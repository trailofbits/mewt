pub mod nodes {
    pub const BINARY_EXPRESSION: &str = "binary_expression";
    pub const BLOCK_ITEM: &str = "block_item";
    pub const BOOL_LITERAL: &str = "bool_literal";
    pub const BREAK_EXPRESSION: &str = "break_expression";
    pub const CALL_EXPRESSION: &str = "call_expression";
    pub const CONTINUE_EXPRESSION: &str = "continue_expression";
    pub const IF_EXPRESSION: &str = "if_expression";
    pub const UNARY_EXPRESSION: &str = "unary_expression";
    pub const WHILE_EXPRESSION: &str = "while_expression";
}

pub mod fields {
    // Condition field in if_expression and while_expression (named "eb" in Sui Move grammar)
    pub const CONDITION: &str = "eb";
    // Arguments field in call_expression
    pub const ARGUMENTS: &str = "args";
    // Unary expression fields
    pub const OPERATOR: &str = "op";
    pub const OPERAND: &str = "expr";
}

#[cfg(test)]
mod tests {
    use tree_sitter::Node;

    use crate::languages::r#move::dialect::profile_for_dialect;
    use crate::types::config::MoveDialect;
    use crate::utils::parse_source;

    use super::{fields, nodes};

    fn first_node_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if let Some(found) = first_node_of_kind(cursor.node(), kind) {
                    return Some(found);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        None
    }

    fn assert_grammar_contract_for_dialect(dialect: MoveDialect) {
        let source = r#"module test::grammar_guard {
    fun check(a: u64, b: u64, cond: bool): u64 {
        let x = a + b;
        let y = !cond;
        let flag = true;
        let z = call_me(a, b, x);

        if (x > 0) {
            if (y) {
                return z
            }
        };

        while (x > 1) {
            if (cond) {
                break;
            } else {
                continue;
            }
        };

        z
    }
}"#;

        let profile = profile_for_dialect(dialect);
        let tree = parse_source(source, profile.parser_language())
            .expect("Move parser should parse grammar guard source");
        let root = tree.root_node();

        let if_expr = first_node_of_kind(root, nodes::IF_EXPRESSION)
            .expect("expected if_expression node in grammar guard source");
        assert!(
            if_expr.child_by_field_name(fields::CONDITION).is_some(),
            "if_expression must expose condition field '{}'",
            fields::CONDITION
        );

        let while_expr = first_node_of_kind(root, nodes::WHILE_EXPRESSION)
            .expect("expected while_expression node in grammar guard source");
        assert!(
            while_expr.child_by_field_name(fields::CONDITION).is_some(),
            "while_expression must expose condition field '{}'",
            fields::CONDITION
        );

        let call_expr = first_node_of_kind(root, nodes::CALL_EXPRESSION)
            .expect("expected call_expression node in grammar guard source");
        assert!(
            call_expr.child_by_field_name(fields::ARGUMENTS).is_some(),
            "call_expression must expose arguments field '{}'",
            fields::ARGUMENTS
        );

        let unary_expr = first_node_of_kind(root, nodes::UNARY_EXPRESSION)
            .expect("expected unary_expression node in grammar guard source");
        assert!(
            unary_expr.child_by_field_name(fields::OPERATOR).is_some(),
            "unary_expression must expose operator field '{}'",
            fields::OPERATOR
        );
        assert!(
            unary_expr.child_by_field_name(fields::OPERAND).is_some(),
            "unary_expression must expose operand field '{}'",
            fields::OPERAND
        );

        first_node_of_kind(root, nodes::BINARY_EXPRESSION)
            .expect("expected binary_expression node in grammar guard source");
        first_node_of_kind(root, nodes::BLOCK_ITEM)
            .expect("expected block_item node in grammar guard source");
        first_node_of_kind(root, nodes::BREAK_EXPRESSION)
            .expect("expected break_expression node in grammar guard source");
        first_node_of_kind(root, nodes::CONTINUE_EXPRESSION)
            .expect("expected continue_expression node in grammar guard source");
        first_node_of_kind(root, nodes::BOOL_LITERAL)
            .expect("expected bool_literal node in grammar guard source");
    }

    #[test]
    fn grammar_contract_holds_for_sui_profile() {
        assert_grammar_contract_for_dialect(MoveDialect::Sui);
    }

    #[test]
    fn grammar_contract_holds_for_iota_profile() {
        assert_grammar_contract_for_dialect(MoveDialect::Iota);
    }
}
