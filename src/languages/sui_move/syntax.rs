pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const BLOCK_ITEM: &str = "block_item";
    pub const BOOL_LITERAL: &str = "bool_literal";
    pub const BREAK_EXPRESSION: &str = "break_expression";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const CONTINUE_EXPRESSION: &str = "continue_expression";
    pub const IF_EXPRESSION: &str = "if_expression";
    pub const WHILE_EXPRESSION: &str = "while_expression";
}

pub mod fields {
    // Condition field in if_expression and while_expression (named "eb" in Sui Move grammar)
    pub const CONDITION: &str = "eb";
    // Arguments field in call_expression
    pub const ARGUMENTS: &str = "args";
}
