use std::path::Path;

use crate::LanguageEngine;
use crate::types::Language;

use super::resolver::{LanguageResolver, ResolutionRequest};

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

    pub fn get_engine(&self, language: &Language) -> Option<&dyn LanguageEngine> {
        self.resolvers
            .iter()
            .flat_map(|resolver| resolver.engines())
            .find(|engine| engine.language() == language)
    }

    pub fn language_from_path(&self, path: &Path) -> Option<&dyn LanguageEngine> {
        self.resolve_engine(ResolutionRequest {
            path,
            explicit_language: None,
            explicit_dialect: None,
            defaults: None,
        })
        .ok()
    }

    pub fn resolve_engine(
        &self,
        request: ResolutionRequest<'_>,
    ) -> Result<&dyn LanguageEngine, String> {
        if request.explicit_dialect.is_some() && request.explicit_language.is_none() {
            let accepting: Vec<_> = self
                .resolvers
                .iter()
                .filter(|resolver| resolver.accepts_cli_dialect())
                .collect();
            if accepting.len() == 1 {
                if let Some(result) = accepting[0].resolve(&request) {
                    return result;
                }
            }
        }

        for resolver in &self.resolvers {
            if let Some(result) = resolver.resolve(&request) {
                return result;
            }
        }

        if let Some(explicit_language) = request.explicit_language {
            return Err(format!(
                "No language resolver found for language: {explicit_language}"
            ));
        }

        let extension = request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_else(|| "<none>".to_string());
        Err(format!(
            "No language resolver found for extension: {extension}"
        ))
    }

    pub fn resolve_canonical_language(
        &self,
        request: ResolutionRequest<'_>,
    ) -> Result<Language, String> {
        self.resolve_engine(request)
            .map(|engine| engine.language().clone())
    }

    pub fn canonicalize_label(&self, raw: &str) -> Option<String> {
        self.filter_labels(raw).into_iter().next()
    }

    pub fn language_supports_cli_dialect_flag(&self, raw: &str) -> bool {
        self.resolvers.iter().any(|resolver| {
            resolver.accepts_cli_dialect()
                && resolver
                    .filter_labels(raw)
                    .is_some_and(|labels| !labels.is_empty())
        })
    }

    pub fn cli_dialect_family(&self) -> Result<Option<&'static str>, String> {
        let accepting: Vec<_> = self
            .resolvers
            .iter()
            .filter(|resolver| resolver.accepts_cli_dialect())
            .map(|resolver| resolver.family())
            .collect();

        match accepting.as_slice() {
            [] => Ok(None),
            [family] => Ok(Some(*family)),
            families => Err(format!(
                "--dialect is ambiguous; accepting language families: {}",
                families.join(", ")
            )),
        }
    }

    pub fn resolve_canonical_for_language_label(
        &self,
        raw_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&super::resolver::ResolutionDefaults>,
    ) -> Result<Language, String> {
        self.resolve_canonical_language(ResolutionRequest {
            path: Path::new("__virtual__.txt"),
            explicit_language: Some(raw_language),
            explicit_dialect,
            defaults,
        })
    }

    pub fn filter_labels(&self, query: &str) -> Vec<String> {
        for resolver in &self.resolvers {
            if let Some(expanded) = resolver.filter_labels(query) {
                return expanded;
            }
        }

        vec![query.to_string()]
    }

    pub fn all_languages(&self) -> Vec<Language> {
        self.resolvers
            .iter()
            .flat_map(|resolver| resolver.engines())
            .map(|engine| engine.language().clone())
            .collect()
    }

    pub fn get_mutation(&self, language: &Language, slug: &str) -> Option<&crate::types::Mutation> {
        let engine = self.get_engine(language)?;
        engine.get_mutations().iter().find(|m| m.slug == slug)
    }

    pub fn get_severity(&self, language: &Language, slug: &str) -> crate::types::MutationSeverity {
        self.get_mutation(language, slug)
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
    use crate::core::resolver::LanguageResolver;
    use crate::languages;

    struct FakeCliDialectResolver;

    impl LanguageResolver for FakeCliDialectResolver {
        fn family(&self) -> &'static str {
            "fake"
        }

        fn engines(&self) -> Vec<&dyn LanguageEngine> {
            Vec::new()
        }

        fn accepts_cli_dialect(&self) -> bool {
            true
        }

        fn resolve<'a>(
            &'a self,
            _request: &crate::core::resolver::ResolutionRequest<'_>,
        ) -> Option<Result<&'a dyn LanguageEngine, String>> {
            None
        }

        fn filter_labels(&self, _query: &str) -> Option<Vec<String>> {
            None
        }
    }

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

    #[test]
    fn cli_dialect_family_reports_none_one_or_ambiguous() {
        let empty = LanguageRegistry::new();
        assert_eq!(empty.cli_dialect_family().unwrap(), None);

        let mut move_only = LanguageRegistry::new();
        move_only.register_resolver(languages::r#move::resolver::MoveLanguageResolver::new());
        assert_eq!(move_only.cli_dialect_family().unwrap(), Some("move"));

        let mut ambiguous = LanguageRegistry::new();
        ambiguous.register_resolver(languages::r#move::resolver::MoveLanguageResolver::new());
        ambiguous.register_resolver(FakeCliDialectResolver);
        let error = ambiguous
            .cli_dialect_family()
            .expect_err("multiple CLI dialect families should be ambiguous");
        assert!(error.contains("--dialect is ambiguous"));
        assert!(error.contains("move"));
        assert!(error.contains("fake"));
    }
}
