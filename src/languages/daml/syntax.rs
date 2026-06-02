// The code reuses the Haskell grammar because DAML's surface syntax is
// Haskell-shaped. DAML-specific constructs (template / choice / controller /
// signatory) are unknown to the grammar and end up inside ERROR-recovered
// subtrees, but the leaf kinds below survive that recovery.

pub mod nodes {
    pub const CONDITIONAL: &str = "conditional";
    pub const INFIX: &str = "infix";
    pub const CONSTRUCTOR: &str = "constructor";
    pub const CONSTRUCTOR_OPERATOR: &str = "constructor_operator";
    pub const VARIABLE: &str = "variable";
}

pub mod fields {
    pub const IF: &str = "if";
}
