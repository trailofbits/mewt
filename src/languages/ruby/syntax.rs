pub mod nodes {
    pub const ARRAY: &str = "array";
    pub const ASSIGNMENT: &str = "assignment";
    pub const BEGIN: &str = "begin";
    pub const BINARY: &str = "binary";
    pub const BREAK: &str = "break";
    pub const CALL: &str = "call";
    pub const CASE: &str = "case";
    pub const CASE_MATCH: &str = "case_match";
    pub const CONDITIONAL: &str = "conditional";
    pub const ELEMENT_REFERENCE: &str = "element_reference";
    pub const ELSIF: &str = "elsif";
    pub const FOR: &str = "for";
    pub const IF: &str = "if";
    pub const IF_GUARD: &str = "if_guard";
    pub const IF_MODIFIER: &str = "if_modifier";
    pub const INTERPOLATION: &str = "interpolation";
    pub const KEYWORD_PARAMETER: &str = "keyword_parameter";
    pub const NEXT: &str = "next";
    pub const OPERATOR_ASSIGNMENT: &str = "operator_assignment";
    pub const OPTIONAL_PARAMETER: &str = "optional_parameter";
    pub const PAIR: &str = "pair";
    pub const REDO: &str = "redo";
    pub const RESCUE_MODIFIER: &str = "rescue_modifier";
    pub const RETRY: &str = "retry";
    pub const RETURN: &str = "return";
    pub const SUPER: &str = "super";
    pub const TRUE: &str = "true";
    pub const FALSE: &str = "false";
    pub const UNARY: &str = "unary";
    pub const UNLESS: &str = "unless";
    pub const UNLESS_GUARD: &str = "unless_guard";
    pub const UNLESS_MODIFIER: &str = "unless_modifier";
    pub const UNTIL: &str = "until";
    pub const UNTIL_MODIFIER: &str = "until_modifier";
    pub const WHILE: &str = "while";
    pub const WHILE_MODIFIER: &str = "while_modifier";
    pub const YIELD: &str = "yield";
    pub const ARRAY_PATTERN: &str = "array_pattern";
    pub const EXPRESSION_REFERENCE_PATTERN: &str = "expression_reference_pattern";
    pub const VARIABLE_REFERENCE_PATTERN: &str = "variable_reference_pattern";
    pub const RANGE: &str = "range";
    pub const STRING: &str = "string";
    pub const HASH: &str = "hash";
}

pub mod fields {
    pub const ARGUMENTS: &str = "arguments";
    pub const CONDITION: &str = "condition";
    pub const OPERAND: &str = "operand";
    pub const OPERATOR: &str = "operator";
    pub const VALUE: &str = "value";
}
