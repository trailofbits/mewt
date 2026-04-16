pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const ASSIGNMENT_EXPRESSION: &str = "assignment_expression";
    pub const UNARY_EXPRESSION: &str = "unary_expression";
    pub const BOOLEAN: &str = "true";
    pub const BOOLEAN_FALSE: &str = "false";
    pub const EXPRESSION_STATEMENT: &str = "expression_statement";
    pub const IF_STATEMENT: &str = "if_statement";
    pub const DECLARATION: &str = "declaration";
    pub const RETURN_STATEMENT: &str = "return_statement";
    pub const WHILE_STATEMENT: &str = "while_statement";
    pub const FOR_STATEMENT: &str = "for_statement";
    pub const FOR_RANGE_LOOP: &str = "for_range_loop";
    pub const DO_STATEMENT: &str = "do_statement";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const BREAK_STATEMENT: &str = "break_statement";
    pub const CONTINUE_STATEMENT: &str = "continue_statement";
}

pub mod fields {
    pub const CONDITION: &str = "condition";
    pub const ARGUMENTS: &str = "arguments";
    pub const OPERATOR: &str = "operator";
    pub const ARGUMENT: &str = "argument";
}
