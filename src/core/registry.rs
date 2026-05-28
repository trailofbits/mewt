use std::path::Path;

use crate::LanguageEngine;

use super::resolver::{LanguageResolver, ResolutionDefaults, ResolutionRequest};

/// Registry for managing language resolvers.
pub struct LanguageRegistry {
    resolvers: Vec<Box<dyn LanguageResolver>>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    pub fn register_resolver<T: LanguageResolver + 'static>(&mut self, resolver: T) {
        self.resolvers.push(Box::new(resolver));
    }

    pub fn get_engine(&self, language_name: &str) -> Option<&dyn LanguageEngine> {
        self.resolvers
            .iter()
            .find(|resolver| {
                resolver.is_language_name(language_name)
                    || resolver
                        .engine()
                        .canonical_name()
                        .eq_ignore_ascii_case(language_name)
                    || resolver.engine().name().eq_ignore_ascii_case(language_name)
            })
            .map(|resolver| resolver.engine())
    }

    pub fn language_from_path(&self, path: &Path) -> Option<&dyn LanguageEngine> {
        let canonical = self
            .resolve_canonical_language(ResolutionRequest {
                path,
                explicit_language: None,
                explicit_dialect: None,
                defaults: None,
            })
            .ok()?;
        self.get_engine(&canonical)
    }

    pub fn resolve_engine(
        &self,
        request: ResolutionRequest<'_>,
    ) -> Result<&dyn LanguageEngine, String> {
        let canonical = self.resolve_canonical_language(request)?;
        self.get_engine(&canonical)
            .ok_or_else(|| format!("No engine found for canonical language: {canonical}"))
    }

    pub fn resolve_canonical_language(
        &self,
        request: ResolutionRequest<'_>,
    ) -> Result<String, String> {
        if let Some(explicit) = request.explicit_language {
            return self.resolve_for_explicit_language(
                explicit,
                request.explicit_dialect,
                request.defaults,
            );
        }

        if let Some(explicit_dialect) = request.explicit_dialect {
            return self.resolve_for_explicit_dialect(explicit_dialect, request.defaults);
        }

        if let Some(defaults) = request.defaults {
            if let Some(default_language) = defaults.default_language.as_deref() {
                return self.resolve_for_explicit_language(default_language, None, Some(defaults));
            }
        }

        let extension = request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| format!("No extension for path: {}", request.path.display()))?;

        for resolver in &self.resolvers {
            if let Some(result) = resolver.resolve_for_extension(extension, request.defaults) {
                return result;
            }
        }

        Err(format!(
            "No language resolver found for extension: .{extension}"
        ))
    }

    fn resolve_for_explicit_dialect(
        &self,
        explicit_dialect: &str,
        defaults: Option<&ResolutionDefaults>,
    ) -> Result<String, String> {
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
    ) -> Result<String, String> {
        for resolver in &self.resolvers {
            if resolver.is_language_name(explicit_language) {
                return resolver.resolve_for_explicit_language(
                    explicit_language,
                    explicit_dialect,
                    defaults,
                );
            }
        }

        Err(format!(
            "No language resolver found for language: {explicit_language}"
        ))
    }

    pub fn canonicalize_label(&self, raw: &str) -> Option<String> {
        for resolver in &self.resolvers {
            if let Some(label) = resolver.canonicalize_label(raw) {
                return Some(label);
            }
        }
        None
    }

    pub fn language_supports_cli_dialect_flag(&self, raw: &str) -> bool {
        self.resolvers
            .iter()
            .any(|resolver| resolver.supports_cli_dialect_flag(raw))
    }

    pub fn resolve_canonical_for_language_label(
        &self,
        raw_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&ResolutionDefaults>,
    ) -> Result<String, String> {
        self.resolve_canonical_language(ResolutionRequest {
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

    pub fn all_languages(&self) -> Vec<&str> {
        self.resolvers
            .iter()
            .map(|resolver| resolver.engine().canonical_name())
            .collect()
    }

    pub fn get_mutation(&self, language_name: &str, slug: &str) -> Option<&crate::types::Mutation> {
        let engine = self.get_engine(language_name)?;
        engine.get_mutations().iter().find(|m| m.slug == slug)
    }

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
    use crate::languages;

    #[test]
    fn cli_dialect_flag_is_move_only_even_when_other_languages_have_dialects() {
        let mut registry = LanguageRegistry::new();
        registry
            .register_resolver(languages::javascript::resolver::JavaScriptLanguageResolver::new());
        registry.register_resolver(languages::r#move::resolver::MoveLanguageResolver::new());

        assert_eq!(
            registry
                .resolve_canonical_for_language_label("javascript/ts", None, None)
                .expect("javascript dialect"),
            "JavaScript/ts"
        );
        assert!(registry.language_supports_cli_dialect_flag("move"));
        assert!(registry.language_supports_cli_dialect_flag("move/iota"));
        assert!(!registry.language_supports_cli_dialect_flag("javascript"));
        assert!(!registry.language_supports_cli_dialect_flag("javascript/ts"));
        assert!(!registry.language_supports_cli_dialect_flag("js"));
    }
}
