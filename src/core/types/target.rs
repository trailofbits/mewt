use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::info;
use serde::Serialize;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::resolver::{ResolutionDefaults, ResolutionRequest};
use crate::types::config::{ResolvedTargets, config, is_path_excluded, is_slug_enabled};
use crate::types::{Hash, Language, Mutant};

#[derive(Debug, Clone, Serialize)]
pub struct Target {
    pub id: i64,
    pub path: PathBuf,
    pub file_hash: Hash,
    pub language: Language,
    #[serde(skip)]
    pub text: String,
}

impl Target {
    /// Returns a cwd-relative path plus resolved language label suitable for logging.
    pub fn display(&self) -> String {
        let path = self.display_path();
        format!("{} ({})", path, self.language)
    }

    /// Returns a cwd-relative path string.
    pub fn display_path(&self) -> String {
        // Try to make the path relative to the current working directory for concise logs
        if let Ok(cwd) = std::env::current_dir() {
            // Ensure we compare absolute paths
            let target_abs = if self.path.is_absolute() {
                self.path.clone()
            } else {
                cwd.join(&self.path)
            };

            if let Ok(relative) = target_abs.strip_prefix(&cwd) {
                let s = relative.to_string_lossy().to_string();
                if s.is_empty() { ".".to_string() } else { s }
            } else {
                self.path.to_string_lossy().to_string()
            }
        } else {
            self.path.to_string_lossy().to_string()
        }
    }

    pub async fn load_targets(
        resolved_targets: &ResolvedTargets,
        store: &SqlStore,
        registry: &LanguageRegistry,
        mutations: Option<&[String]>,
        resolution_defaults: &ResolutionDefaults,
    ) -> io::Result<Vec<Target>> {
        let mut all_targets: Vec<Target> = vec![];

        // Expand globs and collect all target paths
        for pattern in &resolved_targets.include {
            let path = PathBuf::from(pattern);

            if path.is_file() {
                // Direct file reference
                if !is_path_excluded(&path, &resolved_targets.ignore) {
                    if let Some(target) = Self::load_single_file(
                        path,
                        store,
                        registry,
                        mutations,
                        resolution_defaults,
                    )
                    .await?
                    {
                        all_targets.push(target);
                    }
                }
            } else if path.is_dir() {
                // Walk directory
                let targets_from_dir = Box::pin(Self::load_from_directory(
                    path,
                    store,
                    registry,
                    &resolved_targets.ignore,
                    mutations,
                    resolution_defaults,
                ))
                .await?;
                all_targets.extend(targets_from_dir);
            } else {
                // Try as glob pattern
                match glob::glob(pattern) {
                    Ok(paths) => {
                        for entry in paths {
                            match entry {
                                Ok(glob_path) => {
                                    if glob_path.is_file()
                                        && !is_path_excluded(&glob_path, &resolved_targets.ignore)
                                    {
                                        if let Some(target) = Self::load_single_file(
                                            glob_path,
                                            store,
                                            registry,
                                            mutations,
                                            resolution_defaults,
                                        )
                                        .await?
                                        {
                                            all_targets.push(target);
                                        }
                                    } else if glob_path.is_dir() {
                                        let targets_from_dir = Box::pin(Self::load_from_directory(
                                            glob_path,
                                            store,
                                            registry,
                                            &resolved_targets.ignore,
                                            mutations,
                                            resolution_defaults,
                                        ))
                                        .await?;
                                        all_targets.extend(targets_from_dir);
                                    }
                                }
                                Err(e) => {
                                    info!("Skipping invalid glob entry: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("Invalid glob pattern '{}': {}", pattern, e),
                        ));
                    }
                }
            }
        }

        if all_targets.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No valid targets found after filtering",
            ));
        }

        // Note: Targets are already sorted by path from load_single_file's alphabetical
        // traversal of directories, but we sort again to ensure consistent ordering
        // regardless of filesystem traversal order
        all_targets.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(all_targets)
    }

    async fn load_single_file(
        target_path: PathBuf,
        store: &SqlStore,
        registry: &LanguageRegistry,
        _mutations: Option<&[String]>,
        resolution_defaults: &ResolutionDefaults,
    ) -> io::Result<Option<Target>> {
        let mut file = fs::File::open(&target_path)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;

        // Determine language from the file extension.
        // For .move files, use explicitly resolved dialect from config/CLI/per-target config.
        let path_resolution_defaults =
            config().resolve_language_defaults_for_path(&target_path, resolution_defaults);
        let language =
            match resolve_language_for_path(&target_path, registry, &path_resolution_defaults) {
                Some(language) => language,
                None => {
                    info!(
                        "Skipping file {}: unsupported language",
                        target_path.display()
                    );
                    return Ok(None);
                }
            };

        let mut target = Target {
            id: 0, // dummy placeholder until we store it in the db
            path: target_path,
            file_hash: Hash::digest(text.clone()),
            text,
            language,
        };

        match store.add_target(target.clone()).await {
            Ok(id) => {
                target.id = id;
                Ok(Some(target))
            }
            Err(e) => Err(io::Error::other(format!("Failed to store target: {e}"))),
        }
    }

    async fn load_from_directory(
        dir_path: PathBuf,
        store: &SqlStore,
        registry: &LanguageRegistry,
        ignore_patterns: &[String],
        mutations: Option<&[String]>,
        resolution_defaults: &ResolutionDefaults,
    ) -> io::Result<Vec<Target>> {
        // Skip directory entirely if excluded
        if is_path_excluded(&dir_path, ignore_patterns) {
            return Ok(vec![]);
        }

        let mut targets = vec![];
        for entry in fs::read_dir(dir_path)? {
            let path = entry?.path();
            if path.is_file() {
                if !is_path_excluded(&path, ignore_patterns) {
                    if let Some(target) = Self::load_single_file(
                        path,
                        store,
                        registry,
                        mutations,
                        resolution_defaults,
                    )
                    .await?
                    {
                        targets.push(target);
                    }
                }
            } else if path.is_dir() {
                let targets_from_subdir = Box::pin(Self::load_from_directory(
                    path,
                    store,
                    registry,
                    ignore_patterns,
                    mutations,
                    resolution_defaults,
                ))
                .await?;
                targets.extend(targets_from_subdir);
            }
        }
        Ok(targets)
    }

    pub async fn filter_by_path(
        store: &SqlStore,
        target_path: Option<String>,
    ) -> io::Result<Vec<Target>> {
        let mut targets = store.get_all_targets().await.map_err(io::Error::other)?;
        if let Some(path) = target_path {
            // Check if the path contains glob characters
            if path.contains('*') || path.contains('?') || path.contains('[') {
                // Treat as glob pattern
                let glob_pattern = globset::Glob::new(&path)
                    .map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("Invalid glob pattern '{}': {}", path, e),
                        )
                    })?
                    .compile_matcher();

                targets.retain(|t| glob_pattern.is_match(&t.path));
            } else {
                // Exact path match (try canonicalization)
                match PathBuf::from(&path).canonicalize() {
                    Ok(canonical_path) => {
                        targets.retain(|t| t.path == canonical_path);
                    }
                    Err(_) => {
                        // If canonicalization fails (e.g., file doesn't exist),
                        // try matching against the non-canonical path as a fallback
                        let path_buf = PathBuf::from(path);
                        targets.retain(|t| t.path == path_buf);
                    }
                }
            }
        }
        // Targets are already sorted by path from get_all_targets()
        Ok(targets)
    }

    /// Filter existing database targets using include/ignore patterns from ResolvedTargets
    pub async fn filter_existing_by_patterns(
        store: &SqlStore,
        resolved_targets: &ResolvedTargets,
    ) -> io::Result<Vec<Target>> {
        let all_targets = store.get_all_targets().await.map_err(io::Error::other)?;

        // Build globset for include patterns
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &resolved_targets.include {
            if let Ok(glob) = globset::Glob::new(pattern) {
                builder.add(glob);
            }
        }
        let include_matcher = builder.build().ok();

        let mut matched = Vec::new();
        for target in all_targets {
            let is_ignored = is_path_excluded(&target.path, &resolved_targets.ignore);
            let is_included = if let Some(ref matcher) = include_matcher {
                matcher.is_match(&target.path)
            } else {
                false
            };

            if is_included && !is_ignored {
                matched.push(target);
            }
        }

        Ok(matched)
    }

    /// Get targets, using config [targets] if no explicit path is provided
    pub async fn filter_by_path_or_config(
        store: &SqlStore,
        target_path: Option<String>,
    ) -> io::Result<Vec<Target>> {
        // If explicit path provided, use it
        if target_path.is_some() {
            return Self::filter_by_path(store, target_path).await;
        }

        // Otherwise, use config targets if available
        let targets_cfg = config().targets();
        if let Some(cfg) = targets_cfg {
            if let Some(include_patterns) = &cfg.include {
                let resolved = ResolvedTargets {
                    include: include_patterns.clone(),
                    ignore: cfg.ignore.clone().unwrap_or_default(),
                };
                return Self::filter_existing_by_patterns(store, &resolved).await;
            }
        }

        // Fallback: return all targets
        Self::filter_by_path(store, None).await
    }

    pub fn generate_mutants(
        &self,
        registry: &LanguageRegistry,
        mutations: Option<&[String]>,
    ) -> Result<Vec<Mutant>, String> {
        let mut mutants: Vec<Mutant> = Vec::new();

        // Get mutations for this language
        let engine = match registry.get_engine(&self.language) {
            Some(engine) => engine,
            None => return Err(format!("No engine found for language: {}", self.language)),
        };
        let mut new_mutants = engine.mutate(self);

        // Filter by whitelist (if present)
        new_mutants.retain(|m| is_slug_enabled(&m.mutation_slug, mutations));

        mutants.append(&mut new_mutants);

        Ok(mutants)
    }

    pub fn mutate(&self, mutant: &Mutant) -> io::Result<String> {
        if mutant.target_id != self.id && mutant.target_id != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Mutant applies to target {}, not {}",
                    mutant.target_id, self.id
                ),
            ));
        }
        let content_bytes = self.text.as_bytes().to_vec();
        // Replace the text at the specified bytewise position
        let prefix = &content_bytes[..mutant.byte_offset as usize];
        // `len` returns the byte length, `chars` returns the char length, so no as_bytes needed
        let suffix = &content_bytes[(mutant.byte_offset as usize + mutant.old_text.len())..];
        let mutated_content_bytes = [prefix, mutant.new_text.as_bytes(), suffix].concat();
        let mutated_content = String::from_utf8(mutated_content_bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        Ok(mutated_content)
    }

    pub fn restore(&self) -> io::Result<()> {
        std::fs::write(&self.path, &self.text)?;
        Ok(())
    }
}

fn resolve_language_for_path(
    target_path: &Path,
    registry: &LanguageRegistry,
    resolution_defaults: &ResolutionDefaults,
) -> Option<Language> {
    registry
        .resolve_canonical_language(ResolutionRequest {
            path: target_path,
            explicit_language: None,
            defaults: Some(resolution_defaults),
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::resolver::{DialectDefault, ResolutionDefaults};
    use crate::languages;

    fn test_registry() -> LanguageRegistry {
        let mut registry = LanguageRegistry::new();
        registry.register_resolver(languages::rust::resolver::RustLanguageResolver::new());
        registry.register_resolver(languages::r#move::resolver::MoveLanguageResolver::new());
        registry
    }

    fn move_defaults(dialect: &str) -> ResolutionDefaults {
        let mut defaults = ResolutionDefaults::default();
        defaults.default_dialects.insert(
            "move".to_string(),
            DialectDefault {
                dialect: dialect.to_string(),
            },
        );
        defaults
    }

    #[test]
    fn move_paths_use_resolved_dialect_label() {
        let registry = test_registry();
        let language = resolve_language_for_path(
            &PathBuf::from("example.move"),
            &registry,
            &move_defaults("iota"),
        )
        .expect("move language");

        assert_eq!(language, "move/iota");
    }

    #[test]
    fn non_move_paths_use_extension_registry_lookup() {
        let registry = test_registry();
        let language =
            resolve_language_for_path(&PathBuf::from("lib.rs"), &registry, &move_defaults("iota"))
                .expect("rust language");

        assert_eq!(language, "rust");
    }
}
