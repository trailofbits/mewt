use crate::types::{Mutation, MutationSeverity};

pub const CPP_MUTATIONS: &[Mutation] = &[
    Mutation {
        slug: "DAS",
        description: "Delete Array Swap: Swap delete and delete[] to detect scalar/array mismatch (UB)",
        severity: MutationSeverity::High,
    },
    Mutation {
        slug: "MR",
        description: "Move Removal: Remove std::move() wrapper, replacing with its argument",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "VR",
        description: "Virtual Removal: Remove virtual specifier from method declarations",
        severity: MutationSeverity::High,
    },
];
