use crate::LanguageEngine;
use crate::core::registry::{LanguageResolver, ResolutionDefaults};

use super::engine::RustLanguageEngine;

pub struct RustLanguageResolver {
    engine: RustLanguageEngine,
}

impl RustLanguageResolver {
    pub fn new() -> Self {
        Self {
            engine: RustLanguageEngine::new(),
        }
    }
}

impl Default for RustLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for RustLanguageResolver {
    fn engine(&self) -> &dyn LanguageEngine {
        &self.engine
    }

    fn is_language_name(&self, raw: &str) -> bool {
        raw.eq_ignore_ascii_case("rust")
    }

    fn supports_dialect_selection(&self, _raw: &str) -> bool {
        false
    }

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        _defaults: Option<&ResolutionDefaults>,
    ) -> Result<String, String> {
        if explicit_dialect.is_some() {
            return Err("Dialect selection is not supported for Rust".to_string());
        }
        if self.is_language_name(explicit_language) {
            Ok(self.engine.canonical_name().to_string())
        } else {
            Err(format!(
                "No language resolver found for language: {explicit_language}"
            ))
        }
    }

    fn resolve_for_explicit_dialect(
        &self,
        _explicit_dialect: &str,
        _defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<String, String>> {
        None
    }

    fn resolve_for_extension(
        &self,
        extension: &str,
        _defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<String, String>> {
        extension
            .eq_ignore_ascii_case("rs")
            .then(|| Ok(self.engine.canonical_name().to_string()))
    }

    fn canonicalize_label(&self, raw: &str) -> Option<String> {
        self.is_language_name(raw)
            .then(|| self.engine.canonical_name().to_string())
    }

    fn expand_filter_labels(&self, query: &str) -> Option<Vec<String>> {
        self.is_language_name(query)
            .then(|| vec![self.engine.canonical_name().to_string()])
    }
}
