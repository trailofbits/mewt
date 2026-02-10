pub mod nodes {
    pub const BINARY_OPERATOR: &str = "binary_operator";
    pub const SET_ASSIGNMENT: &str = "set_assignment";
    pub const BOOLEAN: &str = "boolean_literal";
    pub const EXPRESSION_STATEMENT: &str = "expression_statement";
    pub const IF_STATEMENT: &str = "if_statement";
    pub const RETURN_STATEMENT: &str = "return_statement";
    pub const THROW_STATEMENT: &str = "throw_statement";
    pub const LOCAL_VARS_DECLARATION: &str = "local_vars_declaration";
    pub const WHILE_STATEMENT: &str = "while_statement";
    pub const DO_WHILE_STATEMENT: &str = "do_while_statement";
    pub const REPEAT_STATEMENT: &str = "repeat_statement";
    pub const FUNCTION_CALL: &str = "function_call";
    pub const BREAK_STATEMENT: &str = "break_statement";
    pub const CONTINUE_STATEMENT: &str = "continue_statement";
}

pub mod fields {
    pub const CONDITION: &str = "condition";
    pub const ARGUMENTS: &str = "arguments";
}
