use std::path::Path;

use crate::LanguageEngine;
use crate::types::Language;

use super::resolver::{DialectPolicy, LanguageResolver, ResolutionRequest};

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
            defaults: None,
        })
        .ok()
    }

    pub fn resolve_engine(
        &self,
        request: ResolutionRequest<'_>,
    ) -> Result<&dyn LanguageEngine, String> {
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

    pub fn dialect_policy(&self, family: &str) -> Option<DialectPolicy> {
        self.resolvers
            .iter()
            .find(|resolver| resolver.family().eq_ignore_ascii_case(family.trim()))
            .map(|resolver| resolver.dialect_policy())
    }

    pub fn validate_dialect_selection(&self, family: &str, dialect: &str) -> Result<(), String> {
        let Some(policy) = self.dialect_policy(family) else {
            return Err(format!("Unknown language family in config: {family}"));
        };

        if !policy.has_dialects() {
            return Err(format!("{} does not support dialect selection", family));
        }

        if !policy.contains(dialect) {
            return Err(format!(
                "Invalid dialect '{}' for {}. Expected one of: {}",
                dialect,
                family,
                policy.expected()
            ));
        }

        Ok(())
    }

    pub fn resolve_canonical_for_language_label(
        &self,
        raw_language: &str,
        defaults: Option<&super::resolver::ResolutionDefaults>,
    ) -> Result<Language, String> {
        self.resolve_canonical_language(ResolutionRequest {
            path: Path::new("__virtual__.txt"),
            explicit_language: Some(raw_language),
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
    use crate::languages;

    #[test]
    fn dialect_policy_reports_language_dialect_keys() {
        let mut registry = LanguageRegistry::new();
        registry
            .register_resolver(languages::javascript::resolver::JavaScriptLanguageResolver::new());
        registry.register_resolver(languages::r#move::resolver::MoveLanguageResolver::new());

        assert_eq!(
            registry
                .resolve_canonical_for_language_label("javascript/ts", None)
                .expect("javascript dialect"),
            "javascript/ts"
        );

        let move_policy = registry.dialect_policy("move").unwrap();
        assert!(move_policy.contains("iota"));
        assert!(!move_policy.contains("tsx"));

        let javascript_policy = registry.dialect_policy("javascript").unwrap();
        assert!(javascript_policy.contains("tsx"));
        assert!(!javascript_policy.contains("iota"));
    }
}
