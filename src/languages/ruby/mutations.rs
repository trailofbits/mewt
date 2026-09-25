use crate::types::{Mutation, MutationSeverity};

pub const RUBY_MUTATIONS: &[Mutation] = &[
    Mutation {
        slug: "UF",
        description: "Unless False: Hardcode an unless condition to false",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "EL",
        description: "Empty Literals: Replace strings, arrays, and hashes with empty equivalents",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "RMOS",
        description: "Regex Match Operator Swap: Swap =~ and !~",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "RBR",
        description: "Range Bound Replacement: Swap inclusive and exclusive range operators",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "CES",
        description: "Case Equality Swap: Swap === and ==",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "SNR",
        description: "Safe Navigation Removal: Remove the &. safe navigation operator",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "UP",
        description: "Unpin Pattern: Remove the ^ pin operator in pattern matching",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "LAOS",
        description: "Logical Assignment Operator Swap: Swap ||= and &&=",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "UT",
        description: "Unless True: Hardcode an unless condition to true",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "ULF",
        description: "Until False: Hardcode an until condition to false",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "ULT",
        description: "Until True: Hardcode an until condition to true",
        severity: MutationSeverity::Low,
    },
];
