use super::dialect::MoveDialect;

#[derive(Debug, Clone, Copy)]
pub struct MoveSyntax {
    pub binary_expression: &'static str,
    pub block_item: Option<&'static str>,
    pub bool_literal: &'static str,
    pub break_expression: &'static str,
    pub call_expression: &'static str,
    pub continue_expression: &'static str,
    pub if_expression: &'static str,
    pub while_expression: &'static str,
    pub condition_field: &'static str,
    pub arguments_field: &'static str,
    pub unary_not_expression: &'static str,
    pub acquires_clause: Option<&'static str>,
    pub unary_operator_field: Option<&'static str>,
    pub unary_operand_field: Option<&'static str>,
}

pub fn syntax_for_dialect(dialect: MoveDialect) -> MoveSyntax {
    match dialect {
        MoveDialect::Sui | MoveDialect::Iota => MoveSyntax {
            binary_expression: "binary_expression",
            block_item: Some("block_item"),
            bool_literal: "bool_literal",
            break_expression: "break_expression",
            call_expression: "call_expression",
            continue_expression: "continue_expression",
            if_expression: "if_expression",
            while_expression: "while_expression",
            condition_field: "eb",
            arguments_field: "args",
            unary_not_expression: "unary_expression",
            acquires_clause: None,
            unary_operator_field: Some("op"),
            unary_operand_field: Some("expr"),
        },
        MoveDialect::Aptos => MoveSyntax {
            binary_expression: "binary_expression",
            block_item: None,
            bool_literal: "bool_literal",
            break_expression: "break_expression",
            call_expression: "call_expression",
            continue_expression: "continue_expression",
            if_expression: "if_expression",
            while_expression: "while_expression",
            condition_field: "condition",
            arguments_field: "arguments",
            unary_not_expression: "not_expression",
            acquires_clause: Some("acquires_clause"),
            unary_operator_field: None,
            unary_operand_field: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use tree_sitter::Node;

    use crate::languages::r#move::dialect::{MoveDialect, config_for_dialect};
    use crate::utils::parse_source;

    use super::syntax_for_dialect;

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

        let dialect_config = config_for_dialect(dialect);
        let syntax = syntax_for_dialect(dialect);
        let tree = parse_source(source, dialect_config.parser_language())
            .expect("Move parser should parse grammar guard source");
        let root = tree.root_node();

        let if_expr = first_node_of_kind(root, syntax.if_expression)
            .expect("expected if_expression node in grammar guard source");
        assert!(
            if_expr
                .child_by_field_name(syntax.condition_field)
                .is_some(),
            "if_expression must expose condition field '{}'",
            syntax.condition_field
        );

        let while_expr = first_node_of_kind(root, syntax.while_expression)
            .expect("expected while_expression node in grammar guard source");
        assert!(
            while_expr
                .child_by_field_name(syntax.condition_field)
                .is_some(),
            "while_expression must expose condition field '{}'",
            syntax.condition_field
        );

        let call_expr = first_node_of_kind(root, syntax.call_expression)
            .expect("expected call_expression node in grammar guard source");
        assert!(
            call_expr
                .child_by_field_name(syntax.arguments_field)
                .is_some(),
            "call_expression must expose arguments field '{}'",
            syntax.arguments_field
        );

        first_node_of_kind(root, syntax.unary_not_expression)
            .expect("expected unary/not expression node in grammar guard source");

        first_node_of_kind(root, syntax.binary_expression)
            .expect("expected binary_expression node in grammar guard source");
        first_node_of_kind(root, syntax.break_expression)
            .expect("expected break_expression node in grammar guard source");
        first_node_of_kind(root, syntax.continue_expression)
            .expect("expected continue_expression node in grammar guard source");
        first_node_of_kind(root, syntax.bool_literal)
            .expect("expected bool_literal node in grammar guard source");
    }

    #[test]
    fn grammar_contract_holds_for_sui_dialect() {
        assert_grammar_contract_for_dialect(MoveDialect::Sui);
    }

    #[test]
    fn grammar_contract_holds_for_iota_dialect() {
        assert_grammar_contract_for_dialect(MoveDialect::Iota);
    }

    #[test]
    fn grammar_contract_holds_for_aptos_dialect() {
        assert_grammar_contract_for_dialect(MoveDialect::Aptos);
    }
}
