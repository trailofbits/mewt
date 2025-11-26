pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const BOOLEAN: &str = "boolean_literal";
    pub const EXPRESSION_STATEMENT: &str = "expression_statement";
    pub const IF_STATEMENT: &str = "if_statement";
    pub const LET_STATEMENT: &str = "variable_declaration_statement";
    pub const METHOD_CALL_EXPRESSION: &str = "function_call";
    pub const STATIC_CALL_EXPRESSION: &str = "function_call";
    pub const RETURN_STATEMENT: &str = "return_statement";
    pub const WHILE_STATEMENT: &str = "while_statement";
    pub const FOR_STATEMENT: &str = "for_statement";
    pub const BREAK_STATEMENT: &str = "break_statement";
    pub const CONTINUE_STATEMENT: &str = "continue_statement";
}

pub mod fields {
    pub const CONDITION: &str = "condition";
    pub const ARGUMENTS: &str = "arguments";
}
