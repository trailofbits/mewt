pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const ASSIGNMENT_STATEMENT: &str = "assignment_statement";
    pub const BOOLEAN: &str = "boolean_literal";
    pub const EXPRESSION_STATEMENT: &str = "expression_statement";
    pub const IF_STATEMENT: &str = "if_statement";
    pub const RETURN_STATEMENT: &str = "return_statement";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const FOR_STATEMENT: &str = "for_statement";
    pub const BREAK_STATEMENT: &str = "break_statement";
    pub const CONTINUE_STATEMENT: &str = "continue_statement";
    pub const SHORT_VAR_DECLARATION: &str = "short_var_declaration";
    pub const INC_STATEMENT: &str = "inc_statement";
    pub const DEC_STATEMENT: &str = "dec_statement";
    pub const UNARY_EXPRESSION: &str = "unary_expression";
    pub const FUNCTION_DECLARATION: &str = "function_declaration";
    pub const METHOD_DECLARATION: &str = "method_declaration";
    pub const FUNC_LITERAL: &str = "func_literal";
    pub const PARAMETER_DECLARATION: &str = "parameter_declaration";
    pub const VARIADIC_PARAMETER_DECLARATION: &str = "variadic_parameter_declaration";
    pub const PARAMETER_LIST: &str = "parameter_list";
    pub const POINTER_TYPE: &str = "pointer_type";
    pub const SLICE_TYPE: &str = "slice_type";
    pub const MAP_TYPE: &str = "map_type";
    pub const CHANNEL_TYPE: &str = "channel_type";
    pub const INTERFACE_TYPE: &str = "interface_type";
    pub const FUNCTION_TYPE: &str = "function_type";
}

pub mod fields {
    pub const CONDITION: &str = "condition";
    pub const ARGUMENTS: &str = "arguments";
    pub const LEFT: &str = "left";
    pub const RIGHT: &str = "right";
    pub const OPERATOR: &str = "operator";
    pub const OPERAND: &str = "operand";
    pub const RESULT: &str = "result";
    pub const TYPE: &str = "type";
}
