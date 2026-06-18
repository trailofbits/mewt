use crate::types::{Mutation, MutationSeverity};

pub const GO_MUTATIONS: &[Mutation] = &[Mutation {
    slug: "DR",
    description: "Defer Removal: Replace defer statement with its deferred call expression",
    severity: MutationSeverity::Medium,
}];
