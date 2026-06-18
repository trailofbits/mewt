use crate::types::{Mutation, MutationSeverity};

pub const DAML_MUTATIONS: &[Mutation] = &[
    Mutation {
        slug: "CPS",
        description: "Controller Party Swap: replace a choice's controller with another Party parameter (from template or choice scope)",
        severity: MutationSeverity::High,
    },
    Mutation {
        slug: "CPR",
        description: "Controller Party Removal: drop one party from a multi-party `controller` list, weakening required authorization",
        severity: MutationSeverity::High,
    },
    Mutation {
        slug: "SPS",
        description: "Signatory Party Swap: replace a template's signatory with another Party parameter from the template's `with`-block",
        severity: MutationSeverity::High,
    },
];
