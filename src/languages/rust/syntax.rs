pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const BOOLEAN: &str = "boolean_literal";
    pub const EXPRESSION_STATEMENT: &str = "expression_statement";
    pub const IF_STATEMENT: &str = "if_expression";
    pub const LET_STATEMENT: &str = "let_declaration";
    pub const METHOD_CALL_EXPRESSION: &str = "call_expression";
    pub const RETURN_STATEMENT: &str = "return_expression";
    pub const STATIC_CALL_EXPRESSION: &str = "call_expression";
    pub const WHILE_STATEMENT: &str = "while_expression";
    pub const BREAK_STATEMENT: &str = "break_expression";
    pub const CONTINUE_STATEMENT: &str = "continue_expression";
    pub const FOREACH_STATEMENT: &str = "for_expression";
}

pub mod fields {
    pub const CONDITION: &str = "condition";
    pub const ARGUMENTS: &str = "arguments";
}
