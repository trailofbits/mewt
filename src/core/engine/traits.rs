use crate::types::{Language, Mutant, Mutation, Target};

/// Core trait that language implementations must provide
pub trait LanguageEngine: Send + Sync {
    /// Concrete language family/dialect handled by this engine.
    fn language(&self) -> &Language;

    /// Get the effective mutation catalog for this concrete engine.
    fn get_mutations(&self) -> &[Mutation];

    /// Apply mutations to a target and return mutants.
    ///
    /// Dialect resolution must already be complete before this method is called.
    /// Implementations may validate `target.language` for diagnostics, but must
    /// not choose dialect config, parser, syntax, or mutation availability by
    /// parsing `target.language` or the target path.
    fn mutate(&self, target: &Target) -> Vec<Mutant>;
}
