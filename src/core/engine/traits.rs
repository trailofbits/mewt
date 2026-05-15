use crate::types::{Mutant, Mutation, Target};

/// Core trait that language implementations must provide
pub trait LanguageEngine: Send + Sync {
    /// User-facing language name (e.g., "Rust", "Iota Move")
    fn name(&self) -> &'static str;

    /// Stable canonical language key used for storage and lookups.
    ///
    /// This is a compatibility contract once persisted in the DB.
    fn canonical_name(&self) -> &'static str {
        self.name()
    }

    /// Get all available mutations for this language
    fn get_mutations(&self) -> &[Mutation];

    /// Apply mutations to a target and return mutants
    fn mutate(&self, target: &Target) -> Vec<Mutant>;
}
