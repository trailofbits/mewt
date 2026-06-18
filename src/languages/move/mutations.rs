use crate::types::{Mutation, MutationSeverity};

pub const MOVE_MUTATIONS: &[Mutation] = &[Mutation {
    slug: "ACQ",
    description: "Acquires Clause Removal: Remove Aptos Move acquires clauses from function declarations",
    severity: MutationSeverity::Medium,
}];
