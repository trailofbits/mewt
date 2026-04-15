use crate::types::{Mutation, MutationSeverity};

pub const SOLIDITY_MUTATIONS: &[Mutation] = &[Mutation {
    slug: "RCI",
    description: "Require Condition Inversion: Invert the condition in require/assert (condition -> !condition)",
    severity: MutationSeverity::Medium,
}];
