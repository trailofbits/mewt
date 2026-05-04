use std::path::Path;

use crate::LanguageEngine;

/// Registry for managing available language engines
pub struct LanguageRegistry {
    engines: Vec<Box<dyn LanguageEngine>>,
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
    /// Move compatibility aliases accepted here:
    /// - move (canonical)
    /// - suimove (legacy)
    /// - sui_move (legacy)
    pub fn get_engine(&self, language_name: &str) -> Option<&dyn LanguageEngine> {
        self.engines
            .iter()
            .find(|engine| {
                engine.name().eq_ignore_ascii_case(language_name)
                    || (is_move_alias(language_name)
                        && (engine.name().eq_ignore_ascii_case("Move")
                            || engine.name().eq_ignore_ascii_case("SuiMove")))
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

fn is_move_alias(language_name: &str) -> bool {
    language_name.eq_ignore_ascii_case("move")
        || language_name.eq_ignore_ascii_case("suimove")
        || language_name.eq_ignore_ascii_case("sui_move")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::sui_move::engine::MoveLanguageEngine;

    #[test]
    fn move_engine_resolves_via_canonical_and_legacy_aliases() {
        let mut registry = LanguageRegistry::new();
        registry.register(MoveLanguageEngine::new());

        assert!(registry.get_engine("move").is_some());
        assert!(registry.get_engine("Move").is_some());
        assert!(registry.get_engine("suimove").is_some());
        assert!(registry.get_engine("SuiMove").is_some());
        assert!(registry.get_engine("sui_move").is_some());
    }
}
