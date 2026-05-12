use std::path::Path;

use crate::LanguageEngine;
use crate::languages::r#move::dialect::{
    dialect_from_language_name, is_move_language_name, language_name_for_dialect,
};
use crate::types::config::ResolvedMoveDialect;

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

pub fn normalize_language_label(language: &str) -> String {
    if let Some(dialect) = dialect_from_language_name(language) {
        language_name_for_dialect(dialect)
    } else {
        language.to_string()
    }
}

pub fn language_filter_variants(language: &str) -> Vec<String> {
    let normalized = language.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "move" => vec![
            "Move".to_string(),
            "Move/sui".to_string(),
            "Move/iota".to_string(),
            "Move/aptos".to_string(),
        ],
        "move/sui" | "move:sui" => vec!["Move/sui".to_string(), "Move".to_string()],
        "move/iota" | "move:iota" => vec!["Move/iota".to_string()],
        "move/aptos" | "move:aptos" => vec!["Move/aptos".to_string()],
        _ => vec![language.to_string()],
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

    pub fn resolve_selection_for_path(
        &self,
        path: &Path,
        explicit_language: Option<&str>,
        resolved_move_dialect: ResolvedMoveDialect,
    ) -> Result<ResolvedLanguageSelection, String> {
        if let Some(explicit) = explicit_language {
            if is_move_language_name(explicit) {
                let canonical_label = language_name_for_dialect(resolved_move_dialect.dialect);
                return Ok(ResolvedLanguageSelection {
                    language_key: "Move".to_string(),
                    dialect: Some(resolved_move_dialect.dialect.as_str().to_string()),
                    canonical_label,
                    source: ResolutionSource::ExplicitLanguage,
                    defaulted: resolved_move_dialect.defaulted,
                });
            }

            let engine = self
                .get_engine(explicit)
                .ok_or_else(|| format!("No engine found for language: {explicit}"))?;
            return Ok(ResolvedLanguageSelection {
                language_key: engine.name().to_string(),
                dialect: None,
                canonical_label: engine.name().to_string(),
                source: ResolutionSource::ExplicitLanguage,
                defaulted: false,
            });
        }

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| format!("No extension for path: {}", path.display()))?;

        if extension.eq_ignore_ascii_case("move") {
            let canonical_label = language_name_for_dialect(resolved_move_dialect.dialect);
            return Ok(ResolvedLanguageSelection {
                language_key: "Move".to_string(),
                dialect: Some(resolved_move_dialect.dialect.as_str().to_string()),
                canonical_label,
                source: ResolutionSource::Extension,
                defaulted: resolved_move_dialect.defaulted,
            });
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
    use crate::languages::r#move::engine::MoveLanguageEngine;
    use crate::types::config::{MoveDialect, MoveDialectSource};
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

    #[test]
    fn move_engine_resolves_via_canonical_names() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());

        assert!(registry.get_engine("move").is_some());
        assert!(registry.get_engine("Move").is_some());
        assert!(registry.get_engine("suimove").is_none());
        assert!(registry.get_engine("SuiMove").is_none());
        assert!(registry.get_engine("sui_move").is_none());
        assert!(registry.get_engine("move/sui").is_some());
        assert!(registry.get_engine("move/iota").is_some());
        assert!(registry.get_engine("move/aptos").is_some());
    }

    fn resolved_move_dialect_iota() -> ResolvedMoveDialect {
        ResolvedMoveDialect {
            dialect: MoveDialect::Iota,
            source: MoveDialectSource::Cli,
            defaulted: false,
        }
    }

    fn resolved_move_dialect_sui() -> ResolvedMoveDialect {
        ResolvedMoveDialect {
            dialect: MoveDialect::Sui,
            source: MoveDialectSource::Config,
            defaulted: false,
        }
    }

    #[test]
    fn resolver_uses_explicit_language_over_path_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(crate::languages::rust::engine::RustLanguageEngine::new());

        let selection = registry
            .resolve_selection_for_path(
                Path::new("example.move"),
                Some("rust"),
                resolved_move_dialect_iota(),
            )
            .expect("selection");

        assert_eq!(selection.canonical_label, "Rust");
        assert_eq!(selection.source, ResolutionSource::ExplicitLanguage);
    }

    #[test]
    fn resolver_canonicalizes_move_with_dialect() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());

        let selection = registry
            .resolve_selection_for_path(
                Path::new("example.move"),
                None,
                resolved_move_dialect_iota(),
            )
            .expect("selection");

        assert_eq!(selection.language_key, "Move");
        assert_eq!(selection.dialect.as_deref(), Some("iota"));
        assert_eq!(selection.canonical_label, "Move/iota");
    }

    #[test]
    fn shared_normalization_helpers_cover_move_variants() {
        assert_eq!(normalize_language_label("Move"), "Move/sui");
        assert_eq!(normalize_language_label("Move/sui"), "Move/sui");
        assert_eq!(language_filter_variants("move").len(), 4);
    }

    #[test]
    fn resolver_selects_move_dialect_deterministically_for_move_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());

        let sui = registry
            .resolve_selection_for_path(
                Path::new("example.move"),
                None,
                resolved_move_dialect_sui(),
            )
            .expect("sui selection");
        let iota = registry
            .resolve_selection_for_path(
                Path::new("example.move"),
                None,
                resolved_move_dialect_iota(),
            )
            .expect("iota selection");

        assert_eq!(sui.canonical_label, "Move/sui");
        assert_eq!(iota.canonical_label, "Move/iota");
        assert_ne!(sui.canonical_label, iota.canonical_label);
    }

    #[test]
    fn resolver_rejects_invalid_move_dialect_language_label() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());

        let err = registry
            .resolve_selection_for_path(
                Path::new("example.move"),
                Some("move/unknown"),
                resolved_move_dialect_sui(),
            )
            .expect_err("invalid dialect label should fail");

        assert!(err.contains("No engine found for language: move/unknown"));
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
            .resolve_selection_for_path(
                Path::new("example.foo"),
                None,
                resolved_move_dialect_iota(),
            )
            .expect("selection");

        assert_eq!(selection.canonical_label, "AlphaLang");
        assert_eq!(selection.source, ResolutionSource::Fallback);
        assert!(selection.defaulted);
    }
}
