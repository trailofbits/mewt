use crate::types::{Mutant, Mutation, MutationSeverity, Target};
use tree_sitter::Language as TsLanguage;

/// Core trait that language implementations must provide
pub trait LanguageEngine: Send + Sync {
    /// Language name (e.g., "Rust", "Solidity")
    fn name(&self) -> &'static str;

    /// File extensions this language handles (e.g., ["rs", "rust"])
    fn extensions(&self) -> &[&'static str];

    /// Get the tree-sitter Language for parsing
    fn tree_sitter_language(&self) -> TsLanguage;

    /// Get all available mutations for this language
    fn get_mutations(&self) -> &[Mutation];

    /// Apply all mutations to a target and return mutants
    fn apply_all_mutations(&self, target: &Target) -> Vec<Mutant>;

    /// Get all unique mutation slugs for this language
    fn get_all_slugs(&self) -> Vec<String> {
        self.get_mutations()
            .iter()
            .map(|m| m.slug.to_string())
            .collect()
    }

    /// Get severity for a mutation slug
    fn get_severity_by_slug(&self, slug: &str) -> Option<MutationSeverity> {
        self.get_mutations()
            .iter()
            .find(|m| m.slug == slug)
            .map(|m| m.severity.clone())
    }
}
