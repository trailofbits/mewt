use crate::types::{Mutation, MutationSeverity};

pub const JAVASCRIPT_MUTATIONS: &[Mutation] = &[
    Mutation {
        slug: "NCR",
        description: "Nullish Coalescing Replacement: Swap ?? and || defaulting operators",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "AWR",
        description: "Await Removal: Replace await expression with the awaited expression",
        severity: MutationSeverity::Medium,
    },
];
