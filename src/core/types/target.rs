use std::fs;
use std::io;
use std::io::Read;
use std::path::PathBuf;

use log::info;
use serde::Serialize;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::config::{ResolvedTargets, is_path_excluded, is_slug_enabled};
use crate::types::{Hash, Mutant};

#[derive(Debug, Clone, Serialize)]
pub struct Target {
    pub id: i64,
    pub path: PathBuf,
    pub file_hash: Hash,
    #[serde(skip)]
    pub text: String,
    pub language: String,
}

impl Target {
    /// Returns a cwd-relative path string suitable for logging
    pub fn display(&self) -> String {
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
    ) -> io::Result<Vec<Target>> {
        let mut all_targets: Vec<Target> = vec![];

        // Expand globs and collect all target paths
        for pattern in &resolved_targets.include {
            let path = PathBuf::from(pattern);

            if path.is_file() {
                // Direct file reference
                if !is_path_excluded(&path, &resolved_targets.ignore)
                    && let Some(target) =
                        Self::load_single_file(path, store, registry, mutations).await?
                {
                    all_targets.push(target);
                }
            } else if path.is_dir() {
                // Walk directory
                let targets_from_dir = Box::pin(Self::load_from_directory(
                    path,
                    store,
                    registry,
                    &resolved_targets.ignore,
                    mutations,
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
                                            glob_path, store, registry, mutations,
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

        Ok(all_targets)
    }

    async fn load_single_file(
        target_path: PathBuf,
        store: &SqlStore,
        registry: &LanguageRegistry,
        _mutations: Option<&[String]>,
    ) -> io::Result<Option<Target>> {
        let mut file = fs::File::open(&target_path)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;

        // Determine language from the file extension
        let language_engine = match registry.language_from_path(&target_path) {
            Some(engine) => engine,
            None => {
                info!(
                    "Skipping file {}: unsupported language",
                    target_path.display()
                );
                return Ok(None);
            }
        };
        let language = language_engine.name().to_string();

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
    ) -> io::Result<Vec<Target>> {
        // Skip directory entirely if excluded
        if is_path_excluded(&dir_path, ignore_patterns) {
            return Ok(vec![]);
        }

        let mut targets = vec![];
        for entry in fs::read_dir(dir_path)? {
            let path = entry?.path();
            if path.is_file() {
                if !is_path_excluded(&path, ignore_patterns)
                    && let Some(target) =
                        Self::load_single_file(path, store, registry, mutations).await?
                {
                    targets.push(target);
                }
            } else if path.is_dir() {
                let targets_from_subdir = Box::pin(Self::load_from_directory(
                    path,
                    store,
                    registry,
                    ignore_patterns,
                    mutations,
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
        let targets = store.get_all_targets().await.map_err(io::Error::other)?;
        if let Some(path) = target_path {
            let path_buf = PathBuf::from(path).canonicalize()?;
            Ok(targets.into_iter().filter(|t| t.path == path_buf).collect())
        } else {
            Ok(targets)
        }
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
        let mut new_mutants = engine.apply_all_mutations(self);

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
