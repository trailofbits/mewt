use std::collections::HashMap;
use std::path::Path;

use crate::LanguageEngine;

/// Registry for managing available language engines and language resolvers.
pub struct LanguageRegistry {
    engines: Vec<Box<dyn LanguageEngine>>,
    resolvers: Vec<Box<dyn LanguageResolver>>,
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

pub trait LanguageResolver {
    fn is_language_name(&self, raw: &str) -> bool;

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&ResolutionDefaults>,
        source: ResolutionSource,
    ) -> Result<ResolvedLanguageSelection, String>;

    fn resolve_for_explicit_dialect(
        &self,
        explicit_dialect: &str,
        defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<ResolvedLanguageSelection, String>>;

    fn resolve_for_extension(
        &self,
        extension: &str,
        defaults: Option<&ResolutionDefaults>,
        has_engine: &dyn Fn(&str) -> bool,
    ) -> Option<Result<ResolvedLanguageSelection, String>>;

    fn canonicalize_label(&self, raw: &str) -> Option<String>;

    fn expand_filter_labels(&self, query: &str) -> Option<Vec<String>>;
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            engines: Vec::new(),
            resolvers: Vec::new(),
        }
    }

    /// Register a language engine.
    pub fn register<T: LanguageEngine + 'static>(&mut self, engine: T) {
        self.engines.push(Box::new(engine));
    }

    /// Register a language resolver.
    pub fn register_resolver<T: LanguageResolver + 'static>(&mut self, resolver: T) {
        self.resolvers.push(Box::new(resolver));
    }

    /// Get engine for a language name.
    pub fn get_engine(&self, language_name: &str) -> Option<&dyn LanguageEngine> {
        self.engines
            .iter()
            .find(|engine| {
                engine.name().eq_ignore_ascii_case(language_name)
                    || self.resolvers.iter().any(|resolver| {
                        resolver.is_language_name(language_name)
                            && resolver.is_language_name(engine.name())
                    })
            })
            .map(|engine| engine.as_ref())
    }

    /// Determine language from file path.
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

        for resolver in &self.resolvers {
            if let Some(result) =
                resolver.resolve_for_extension(extension, request.defaults, &|name| {
                    self.get_engine(name).is_some()
                })
            {
                return result;
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
        for resolver in &self.resolvers {
            if let Some(result) = resolver.resolve_for_explicit_dialect(explicit_dialect, defaults)
            {
                return result;
            }
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
        for resolver in &self.resolvers {
            if resolver.is_language_name(explicit_language) {
                return resolver.resolve_for_explicit_language(
                    explicit_language,
                    explicit_dialect,
                    defaults,
                    source,
                );
            }
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
        for resolver in &self.resolvers {
            if let Some(label) = resolver.canonicalize_label(raw) {
                return Some(label);
            }
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
        for resolver in &self.resolvers {
            if let Some(expanded) = resolver.expand_filter_labels(query) {
                return expanded;
            }
        }

        self.canonicalize_label(query)
            .map(|label| vec![label])
            .unwrap_or_else(|| vec![query.to_string()])
    }

    /// Get all registered language names.
    pub fn all_languages(&self) -> Vec<&str> {
        self.engines.iter().map(|engine| engine.name()).collect()
    }

    /// Get a specific mutation by language and slug.
    pub fn get_mutation(&self, language_name: &str, slug: &str) -> Option<&crate::types::Mutation> {
        let engine = self.get_engine(language_name)?;
        engine.get_mutations().iter().find(|m| m.slug == slug)
    }

    /// Get severity for a mutation, defaults to Low if not found.
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
    use crate::languages::javascript::resolver::JavaScriptLanguageResolver;
    use crate::languages::r#move::engine::MoveLanguageEngine;
    use crate::languages::r#move::resolver::MoveLanguageResolver;
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
        registry.register_resolver(MoveLanguageResolver::new());
        assert!(registry.get_engine("move").is_some());
        assert!(registry.get_engine("move/iota").is_some());
    }

    #[test]
    fn resolver_uses_explicit_language_over_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(crate::languages::rust::engine::RustLanguageEngine::new());
        registry.register_resolver(MoveLanguageResolver::new());

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
        registry.register_resolver(MoveLanguageResolver::new());

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
        registry.register_resolver(MoveLanguageResolver::new());

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
        registry.register_resolver(JavaScriptLanguageResolver::new());

        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("component.tsx"),
                explicit_language: None,
                explicit_dialect: None,
                defaults: None,
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "JavaScript/tsx");
        assert_eq!(selection.dialect.as_deref(), Some("tsx"));
        assert_eq!(selection.source, ResolutionSource::Extension);
        assert!(!selection.defaulted);
    }

    #[test]
    fn resolver_explicit_language_overrides_js_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(JavaScriptLanguageEngine::new());
        registry.register(crate::languages::rust::engine::RustLanguageEngine::new());
        registry.register_resolver(JavaScriptLanguageResolver::new());

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

    #[test]
    fn resolver_explicit_js_dialect_overrides_extension_default() {
        let mut registry = LanguageRegistry::new();
        registry.register(JavaScriptLanguageEngine::new());
        registry.register_resolver(JavaScriptLanguageResolver::new());

        let selection = registry
            .resolve_selection(ResolutionRequest {
                path: Path::new("component.js"),
                explicit_language: Some("javascript"),
                explicit_dialect: Some("tsx"),
                defaults: None,
            })
            .expect("selection");

        assert_eq!(selection.canonical_label, "JavaScript/tsx");
        assert_eq!(selection.dialect.as_deref(), Some("tsx"));
        assert_eq!(selection.source, ResolutionSource::ExplicitLanguage);
    }

    #[test]
    fn javascript_family_filters_expand_to_all_dialects() {
        let mut registry = LanguageRegistry::new();
        registry.register(JavaScriptLanguageEngine::new());
        registry.register_resolver(JavaScriptLanguageResolver::new());

        let labels = registry.filter_labels("javascript");
        assert_eq!(
            labels,
            vec![
                "JavaScript/js".to_string(),
                "JavaScript/jsx".to_string(),
                "JavaScript/ts".to_string(),
                "JavaScript/tsx".to_string()
            ]
        );
    }
}
