use crate::types::{Mutant, Mutation, Target};

/// Core trait that language implementations must provide
pub trait LanguageEngine: Send + Sync {
    /// User-facing language name (e.g., "Rust", "Iota Move")
    fn name(&self) -> &'static str;

    /// Stable canonical language key used for storage and lookups.
    ///
    /// This is a compatibility contract once persisted in the DB. Dialect-aware
    /// engines must return a concrete dialect label such as `Move/iota` or
    /// `JavaScript/tsx`; one concrete engine should map to one canonical label.
    fn canonical_name(&self) -> &'static str {
        self.name()
    }

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
