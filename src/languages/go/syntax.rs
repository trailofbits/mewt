pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const BOOLEAN: &str = "boolean_literal";
    pub const EXPRESSION_STATEMENT: &str = "expression_statement";
    pub const IF_STATEMENT: &str = "if_statement";
    pub const RETURN_STATEMENT: &str = "return_statement";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const FOR_STATEMENT: &str = "for_statement";
    pub const BREAK_STATEMENT: &str = "break_statement";
    pub const CONTINUE_STATEMENT: &str = "continue_statement";
    pub const UNARY_EXPRESSION: &str = "unary_expression";
}

pub mod fields {
    pub const CONDITION: &str = "condition";
    pub const ARGUMENTS: &str = "arguments";
    pub const LEFT: &str = "left";
    pub const RIGHT: &str = "right";
    pub const OPERATOR: &str = "operator";
}
