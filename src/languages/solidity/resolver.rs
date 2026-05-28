use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionDefaults};

use super::engine::SolidityLanguageEngine;

pub struct SolidityLanguageResolver {
    engine: SolidityLanguageEngine,
}

impl SolidityLanguageResolver {
    pub fn new() -> Self {
        Self {
            engine: SolidityLanguageEngine::new(),
        }
    }
}

impl Default for SolidityLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for SolidityLanguageResolver {
    fn engine(&self) -> &dyn LanguageEngine {
        &self.engine
    }

    fn is_language_name(&self, raw: &str) -> bool {
        raw.eq_ignore_ascii_case("solidity") || raw.eq_ignore_ascii_case("sol")
    }

    fn supports_cli_dialect_flag(&self, _raw: &str) -> bool {
        false
    }

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        _defaults: Option<&ResolutionDefaults>,
    ) -> Result<String, String> {
        if explicit_dialect.is_some() {
            return Err("Dialect selection is not supported for Solidity".to_string());
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
            .eq_ignore_ascii_case("sol")
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
