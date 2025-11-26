use std::path::Path;
use tree_sitter::Tree;

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

    /// Get engine for a language name
    pub fn get_engine(&self, language_name: &str) -> Option<&dyn LanguageEngine> {
        self.engines
            .iter()
            .find(|engine| engine.name().eq_ignore_ascii_case(language_name))
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

    /// Parse source code with the appropriate language
    pub fn parse(&self, language_name: &str, source: &str) -> Option<Tree> {
        let engine = self.get_engine(language_name)?;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&engine.tree_sitter_language()).ok()?;
        parser.parse(source, None)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
