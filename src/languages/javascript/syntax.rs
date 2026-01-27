pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const EXPRESSION_STATEMENT: &str = "expression_statement";
    pub const IF_STATEMENT: &str = "if_statement";
    pub const WHILE_STATEMENT: &str = "while_statement";
    pub const FOR_STATEMENT: &str = "for_statement";
    pub const FOR_IN_STATEMENT: &str = "for_in_statement";
    pub const DO_STATEMENT: &str = "do_statement";
    pub const RETURN_STATEMENT: &str = "return_statement";
    pub const VARIABLE_DECLARATION: &str = "variable_declaration";
    pub const BREAK_STATEMENT: &str = "break_statement";
    pub const CONTINUE_STATEMENT: &str = "continue_statement";
}

pub mod fields {
    pub const CONDITION: &str = "condition";
    pub const ARGUMENTS: &str = "arguments";
}
