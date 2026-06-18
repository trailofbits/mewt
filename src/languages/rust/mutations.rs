use crate::types::{Mutation, MutationSeverity};

pub const RUST_MUTATIONS: &[Mutation] = &[Mutation {
    slug: "RBR",
    description: "Range Boundary Replacement: Swap exclusive and inclusive range operators (.. <-> ..=)",
    severity: MutationSeverity::Medium,
}];
