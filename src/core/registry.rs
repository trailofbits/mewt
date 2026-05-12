use std::collections::HashMap;
use std::path::Path;

use crate::LanguageEngine;
use crate::languages::r#move::dialect::{
    dialect_from_language_name, is_move_language_name, language_name_for_dialect,
};
/// Registry for managing available language engines
pub struct LanguageRegistry {
    engines: Vec<Box<dyn LanguageEngine>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionSource {
    ExplicitLanguage,
    Extension,
    Fallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLanguageSelection {
    pub language_key: String,
    pub dialect: Option<String>,
    pub canonical_label: String,
    pub source: ResolutionSource,
    pub defaulted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolutionDefaults {
    pub default_language: Option<String>,
    pub default_dialects: HashMap<String, DialectDefault>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialectDefault {
    pub dialect: String,
    pub defaulted: bool,
}

pub struct ResolutionRequest<'a> {
    pub path: &'a Path,
    pub explicit_language: Option<&'a str>,
    pub explicit_dialect: Option<&'a str>,
    pub defaults: Option<&'a ResolutionDefaults>,
}

pub fn canonicalize_language_label(language: &str) -> String {
    if let Some(dialect) = dialect_from_language_name(language) {
        language_name_for_dialect(dialect)
    } else {
        language.to_string()
    }
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
        }
    }

    /// Register a language engine
    pub fn register<T: LanguageEngine + 'static>(&mut self, engine: T) {
        self.engines.push(Box::new(engine));
    }

    /// Get engine for a language name.
    ///
    /// Move names accepted here:
    /// - move (canonical selector)
    /// - move/sui, move/iota, move/aptos (profiled names)
    pub fn get_engine(&self, language_name: &str) -> Option<&dyn LanguageEngine> {
        self.engines
            .iter()
            .find(|engine| {
                engine.name().eq_ignore_ascii_case(language_name)
                    || (is_move_language_name(language_name)
                        && is_move_language_name(engine.name()))
            })
            .map(|engine| engine.as_ref())
    }

    /// Determine language from file path
    pub fn language_from_path(&self, path: &Path) -> Option<&dyn LanguageEngine> {
        let extension = path.extension().and_then(|ext| ext.to_str())?;

        self.engines
            .iter()
            .find(|engine| {
                engine
                    .extensions()
                    .iter()
                    .any(|ext| ext.eq_ignore_ascii_case(extension))
            })
            .map(|engine| engine.as_ref())
    }

    pub fn resolve_selection(
        &self,
        request: ResolutionRequest<'_>,
    ) -> Result<ResolvedLanguageSelection, String> {
        if let Some(explicit) = request.explicit_language {
            return self.resolve_for_explicit_language(
                explicit,
                request.explicit_dialect,
                request.defaults,
                ResolutionSource::ExplicitLanguage,
            );
        }

        if let Some(explicit_dialect) = request.explicit_dialect {
            return self.resolve_for_explicit_dialect(explicit_dialect, request.defaults);
        }

        if let Some(defaults) = request.defaults {
            if let Some(default_language) = defaults.default_language.as_deref() {
                return self.resolve_for_explicit_language(
                    default_language,
                    None,
                    Some(defaults),
                    ResolutionSource::Fallback,
                );
            }
        }

        let extension = request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| format!("No extension for path: {}", request.path.display()))?;

        if extension.eq_ignore_ascii_case("move") {
            if let Some(defaults) = request.defaults {
                if let Some(default_move_dialect) = defaults.default_dialects.get("move") {
                    if let Some(dialect) = dialect_from_language_name(&format!(
                        "move/{}",
                        default_move_dialect.dialect
                    )) {
                        return Ok(ResolvedLanguageSelection {
                            language_key: "Move".to_string(),
                            dialect: Some(dialect.as_str().to_string()),
                            canonical_label: language_name_for_dialect(dialect),
                            source: ResolutionSource::Extension,
                            defaulted: default_move_dialect.defaulted,
                        });
                    }
                }
            }
        }

        let mut candidates: Vec<&dyn LanguageEngine> = self
            .engines
            .iter()
            .filter_map(|engine| {
                if engine
                    .extensions()
                    .iter()
                    .any(|ext| ext.eq_ignore_ascii_case(extension))
                {
                    Some(engine.as_ref())
                } else {
                    None
                }
            })
            .collect();

        if candidates.is_empty() {
            return Err(format!(
                "No language engine found for extension: .{extension}"
            ));
        }

        candidates.sort_by(|a, b| a.name().cmp(b.name()));
        let selected = candidates[0];
        let defaulted = candidates.len() > 1;

        Ok(ResolvedLanguageSelection {
            language_key: selected.name().to_string(),
            dialect: None,
            canonical_label: selected.name().to_string(),
            source: if defaulted {
                ResolutionSource::Fallback
            } else {
                ResolutionSource::Extension
            },
            defaulted,
        })
    }

    fn resolve_for_explicit_dialect(
        &self,
        explicit_dialect: &str,
        defaults: Option<&ResolutionDefaults>,
    ) -> Result<ResolvedLanguageSelection, String> {
        if let Some(dialect) = dialect_from_language_name(&format!("move/{explicit_dialect}")) {
            return Ok(ResolvedLanguageSelection {
                language_key: "Move".to_string(),
                dialect: Some(dialect.as_str().to_string()),
                canonical_label: language_name_for_dialect(dialect),
                source: ResolutionSource::ExplicitLanguage,
                defaulted: false,
            });
        }

        if let Some(defaults) = defaults {
            if let Some(default_language) = defaults.default_language.as_deref() {
                return self.resolve_for_explicit_language(
                    default_language,
                    Some(explicit_dialect),
                    Some(defaults),
                    ResolutionSource::ExplicitLanguage,
                );
            }
        }

        Err(format!(
            "No language family found for dialect: {explicit_dialect}"
        ))
    }

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&ResolutionDefaults>,
        source: ResolutionSource,
    ) -> Result<ResolvedLanguageSelection, String> {
        if is_move_language_name(explicit_language) {
            let dialect = if let Some(raw) = explicit_dialect {
                dialect_from_language_name(&format!("move/{raw}")).ok_or_else(|| {
                    format!("Invalid Move dialect '{raw}'. Expected one of: sui, iota, aptos")
                })?
            } else if let Some(from_label) = dialect_from_language_name(explicit_language) {
                from_label
            } else if let Some(defaults) = defaults {
                defaults
                    .default_dialects
                    .get("move")
                    .and_then(|entry| {
                        dialect_from_language_name(&format!("move/{}", entry.dialect))
                    })
                    .unwrap_or(crate::types::config::MoveDialect::Sui)
            } else {
                crate::types::config::MoveDialect::Sui
            };

            let defaulted = explicit_dialect.is_none()
                && defaults
                    .and_then(|d| d.default_dialects.get("move"))
                    .is_some_and(|entry| entry.defaulted);

            return Ok(ResolvedLanguageSelection {
                language_key: "Move".to_string(),
                dialect: Some(dialect.as_str().to_string()),
                canonical_label: language_name_for_dialect(dialect),
                source,
                defaulted,
            });
        }

        let engine = self
            .get_engine(explicit_language)
            .ok_or_else(|| format!("No engine found for language: {explicit_language}"))?;
        Ok(ResolvedLanguageSelection {
            language_key: engine.name().to_string(),
            dialect: None,
            canonical_label: engine.name().to_string(),
            source,
            defaulted: false,
        })
    }

    pub fn canonicalize_label(&self, raw: &str) -> Option<String> {
        if is_move_language_name(raw) {
            return Some(canonicalize_language_label(raw));
        }

        self.get_engine(raw).map(|engine| engine.name().to_string())
    }

    pub fn language_supports_dialect_selection(&self, raw: &str) -> bool {
        self.resolve_selection_for_language_label(raw, None, None)
            .map(|selection| selection.dialect.is_some())
            .unwrap_or(false)
    }

    pub fn resolve_selection_for_language_label(
        &self,
        raw_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&ResolutionDefaults>,
    ) -> Result<ResolvedLanguageSelection, String> {
        self.resolve_selection(ResolutionRequest {
            path: Path::new("__virtual__.txt"),
            explicit_language: Some(raw_language),
            explicit_dialect,
            defaults,
        })
    }

    pub fn filter_labels(&self, query: &str) -> Vec<String> {
        if let Some(expanded) = self.expand_family_filter_labels(query) {
            return expanded;
        }

        self.canonicalize_label(query)
            .map(|label| vec![label])
            .unwrap_or_else(|| vec![query.to_string()])
    }

    fn expand_family_filter_labels(&self, query: &str) -> Option<Vec<String>> {
        let normalized = query.trim().to_ascii_lowercase();

        if !is_move_language_name(&normalized) {
            return None;
        }

        let move_profiles = ["sui", "iota", "aptos"];

        if normalized == "move" {
            let mut labels = vec!["Move".to_string()];
            labels.extend(
                move_profiles
                    .iter()
                    .map(|profile| format!("Move/{profile}")),
            );
            return Some(labels);
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if move_profiles.contains(&dialect) {
            if dialect == "sui" {
                return Some(vec!["Move/sui".to_string(), "Move".to_string()]);
            }
            return Some(vec![format!("Move/{dialect}")]);
        }

        None
    }

    /// Get all registered language names
    pub fn all_languages(&self) -> Vec<&str> {
        self.engines.iter().map(|engine| engine.name()).collect()
    }

    /// Get a specific mutation by language and slug
    pub fn get_mutation(&self, language_name: &str, slug: &str) -> Option<&crate::types::Mutation> {
        let engine = self.get_engine(language_name)?;
        engine.get_mutations().iter().find(|m| m.slug == slug)
    }

    /// Get severity for a mutation, defaults to Low if not found
    pub fn get_severity(&self, language_name: &str, slug: &str) -> crate::types::MutationSeverity {
        self.get_mutation(language_name, slug)
            .map(|m| m.severity.clone())
            .unwrap_or(crate::types::MutationSeverity::Low)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::javascript::engine::JavaScriptLanguageEngine;
    use crate::languages::r#move::engine::MoveLanguageEngine;
    use crate::types::{Mutant, Mutation, Target};

    struct MockEngine {
        name: &'static str,
        exts: &'static [&'static str],
    }

    impl crate::LanguageEngine for MockEngine {
        fn name(&self) -> &'static str {
            self.name
        }
        fn extensions(&self) -> &[&'static str] {
            self.exts
        }
        fn get_mutations(&self) -> &[Mutation] {
            &[]
        }
        fn mutate(&self, _target: &Target) -> Vec<Mutant> {
            vec![]
        }
    }

    fn move_defaults(dialect: &str, defaulted: bool) -> ResolutionDefaults {
        let mut defaults = ResolutionDefaults::default();
        defaults.default_dialects.insert(
            "move".to_string(),
            DialectDefault {
                dialect: dialect.to_string(),
                defaulted,
            },
        );
        defaults
    }

    #[test]
    fn move_engine_resolves_via_canonical_names() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());
        assert!(registry.get_engine("move").is_some());
        assert!(registry.get_engine("move/iota").is_some());
    }

    #[test]
    fn resolver_uses_explicit_language_over_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(crate::languages::rust::engine::RustLanguageEngine::new());

        let defaults = move_defaults("iota", false);
        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("example.move"),
                explicit_language: Some("rust"),
                explicit_dialect: None,
                defaults: Some(&defaults),
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "Rust");
    }

    #[test]
    fn resolver_uses_default_dialect_for_move_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());

        let defaults = move_defaults("iota", true);
        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("example.move"),
                explicit_language: None,
                explicit_dialect: None,
                defaults: Some(&defaults),
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "Move/iota");
        assert!(selection.defaulted);
    }

    #[test]
    fn resolver_uses_explicit_dialect_precedence() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());

        let defaults = move_defaults("sui", true);
        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("example.move"),
                explicit_language: None,
                explicit_dialect: Some("aptos"),
                defaults: Some(&defaults),
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "Move/aptos");
        assert!(!selection.defaulted);
    }

    #[test]
    fn resolver_uses_deterministic_fallback_for_ambiguous_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(MockEngine {
            name: "BetaLang",
            exts: &["foo"],
        });
        registry.register(MockEngine {
            name: "AlphaLang",
            exts: &["foo"],
        });

        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("example.foo"),
                explicit_language: None,
                explicit_dialect: None,
                defaults: None,
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "AlphaLang");
        assert_eq!(selection.source, ResolutionSource::Fallback);
        assert!(selection.defaulted);
    }

    #[test]
    fn resolver_uses_javascript_engine_for_js_family_extensions() {
        let mut registry = LanguageRegistry::new();
        registry.register(JavaScriptLanguageEngine::new());

        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("component.tsx"),
                explicit_language: None,
                explicit_dialect: None,
                defaults: None,
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "JavaScript");
        assert_eq!(selection.source, ResolutionSource::Extension);
        assert!(!selection.defaulted);
    }

    #[test]
    fn resolver_explicit_language_overrides_js_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(JavaScriptLanguageEngine::new());
        registry.register(crate::languages::rust::engine::RustLanguageEngine::new());

        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("component.tsx"),
                explicit_language: Some("rust"),
                explicit_dialect: None,
                defaults: None,
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "Rust");
        assert_eq!(selection.source, ResolutionSource::ExplicitLanguage);
        assert!(!selection.defaulted);
    }
}
