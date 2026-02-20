use crate::types::{Mutant, Mutation, Target};

/// Core trait that language implementations must provide
pub trait LanguageEngine: Send + Sync {
    /// Language name (e.g., "Rust", "Solidity")
    fn name(&self) -> &'static str;

    /// File extensions this language handles (e.g., ["rs", "rust"])
    fn extensions(&self) -> &[&'static str];

    /// Get all available mutations for this language
    fn get_mutations(&self) -> &[Mutation];

    /// Apply mutations to a target and return mutants
    fn mutate(&self, target: &Target) -> Vec<Mutant>;
}
