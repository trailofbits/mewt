use crate::types::{Mutation, MutationSeverity};

pub const COMMON_MUTATIONS: &[Mutation] = &[
    Mutation {
        slug: "ER",
        description: "Error Replacement: Replace statement with an error",
        severity: MutationSeverity::High,
    },
    Mutation {
        slug: "CR",
        description: "Comment Replacement: Replace statement with an in-line comment",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "IF",
        description: "If False: Hardcode an if condition to false",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "IT",
        description: "If True: Hardcode an if condition to true",
        severity: MutationSeverity::Medium,
    },
    Mutation {
        slug: "WF",
        description: "While False: Hardcode while condition to false",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "AS",
        description: "Argument Swap: swap pairs of adjacent arguments",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "LC",
        description: "Loop Control: Swap break and continue statements",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "BL",
        description: "Boolean Literal Flip: true <-> false",
        severity: MutationSeverity::Low,
    },
    // Shared operator shuffle mutations
    Mutation {
        slug: "AOS",
        description: "Arithmetic Operator Shuffle: Replace arithmetic operators (+, -, *, /)",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "AAOS",
        description: "Arithmetic Assignment Operator Shuffle: Replace arithmetic assignment operators (+=, -=, *=, /=)",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "BOS",
        description: "Bitwise Operator Shuffle: Replace bitwise operators (&, |, ^)",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "BAOS",
        description: "Bitwise Assignment Operator Shuffle: Replace bitwise assignment operators (&=, |=, ^=)",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "LOS",
        description: "Logical Operator Shuffle: Replace logical operators (&&, ||)",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "COS",
        description: "Comparison Operator Shuffle: Replace comparison operators (==, !=, <, <=, >, >=)",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "SOS",
        description: "Shift Operator Shuffle: Replace shift operators (<<, >>)",
        severity: MutationSeverity::Low,
    },
    Mutation {
        slug: "SAOS",
        description: "Shift Assignment Operator Shuffle: Replace shift assignment operators (<<=, >>=)",
        severity: MutationSeverity::Low,
    },
];
