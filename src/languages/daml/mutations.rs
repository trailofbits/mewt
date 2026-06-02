use crate::types::{Mutation, MutationSeverity};

pub const DAML_MUTATIONS: &[Mutation] = &[
    Mutation {
        slug: "CPS",
        description: "Controller Party Swap: replace a choice's controller with another Party parameter of the same template",
        severity: MutationSeverity::High,
    },
    Mutation {
        slug: "CPR",
        description: "Controller Party Removal: drop one party from a multi-party `controller` list, weakening required authorization",
        severity: MutationSeverity::High,
    },
];
