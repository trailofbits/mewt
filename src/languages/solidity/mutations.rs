use crate::types::{Mutation, MutationSeverity};

pub const SOLIDITY_MUTATIONS: &[Mutation] = &[Mutation {
    slug: "RDV",
    description: "Return Default Value: Replace return value with type-appropriate default",
    severity: MutationSeverity::Medium,
}];
