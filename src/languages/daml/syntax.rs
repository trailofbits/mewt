// Node and field names from the vendored tree-sitter-daml grammar
// (`grammars/daml/`). The grammar extends upstream tree-sitter-haskell with
// DAML's template / choice / signatory / controller declarations, plus a
// `:` / `::` swap so DAML's single-colon type annotation surfaces as a
// typed structure.
//
// Only the node kinds and field names the engine actually traverses live
// here.

pub mod nodes {
    // Common Haskell-shaped kinds reused by the shared mutation patterns
    // (IF, IT, BL, AOS, COS, LOS). Identical names to upstream
    // tree-sitter-haskell.
    pub const CONDITIONAL: &str = "conditional";
    pub const INFIX: &str = "infix";
    pub const CONSTRUCTOR: &str = "constructor";
    pub const VARIABLE: &str = "variable";

    // DAML-specific structural nodes
    pub const TEMPLATE: &str = "template";
    pub const CHOICE_DECL: &str = "choice_decl";
    pub const CONTROLLER_DECL: &str = "controller_decl";
    pub const SIGNATORY_DECL: &str = "signatory_decl";
    pub const FIELD_DECL: &str = "field_decl";
    // A module-qualified identifier like `M.Party`; has fields `module` and `id`.
    pub const QUALIFIED: &str = "qualified";
    // A bare identifier token (named leaf, no fields), e.g. the `id` side of `M.Party`.
    pub const NAME: &str = "name";
}

// node = an AST element identified by `kind()`; field = a labelled child reached
// via `child_by_field_name()`.

pub mod fields {
    // `conditional`'s if-field, for IF / IT.
    pub const IF: &str = "if";

    // `controller_decl.party`, `field_decl.type`, `field_decl.name`,
    // `template.fields`, and `with_fields.field` for CPS / CPR scope.
    pub const PARTY: &str = "party";
    pub const NAME: &str = "name";
    pub const TYPE: &str = "type";
    pub const FIELDS: &str = "fields";
    pub const FIELD: &str = "field";
    pub const ID: &str = "id";
}
