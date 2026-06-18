use super::dialect::JavaScriptDialect;

#[derive(Debug, Clone, Copy)]
pub struct JavaScriptSyntax {
    pub binary_expression: &'static str,
    pub augmented_assignment_expression: &'static str,
    pub call_expression: &'static str,
    pub expression_statement: &'static str,
    pub if_statement: &'static str,
    pub while_statement: &'static str,
    pub for_statement: &'static str,
    pub for_in_statement: &'static str,
    pub do_statement: &'static str,
    pub return_statement: &'static str,
    pub variable_declaration: &'static str,
    pub break_statement: &'static str,
    pub continue_statement: &'static str,
    pub unary_expression: &'static str,
    pub type_arguments: Option<&'static str>,
    pub type_parameters: Option<&'static str>,
    pub type_assertion: Option<&'static str>,
    pub jsx_element: Option<&'static str>,
    pub condition_field: &'static str,
    pub arguments_field: &'static str,
    pub operator_field: &'static str,
    pub argument_field: &'static str,
}

pub fn syntax_for_dialect(dialect: JavaScriptDialect) -> JavaScriptSyntax {
    let common = JavaScriptSyntax {
        binary_expression: "binary_expression",
        augmented_assignment_expression: "augmented_assignment_expression",
        call_expression: "call_expression",
        expression_statement: "expression_statement",
        if_statement: "if_statement",
        while_statement: "while_statement",
        for_statement: "for_statement",
        for_in_statement: "for_in_statement",
        do_statement: "do_statement",
        return_statement: "return_statement",
        variable_declaration: "variable_declaration",
        break_statement: "break_statement",
        continue_statement: "continue_statement",
        unary_expression: "unary_expression",
        type_arguments: None,
        type_parameters: None,
        type_assertion: None,
        jsx_element: None,
        condition_field: "condition",
        arguments_field: "arguments",
        operator_field: "operator",
        argument_field: "argument",
    };

    match dialect {
        JavaScriptDialect::JavaScript => JavaScriptSyntax {
            jsx_element: Some("jsx_element"),
            ..common
        },
        JavaScriptDialect::Jsx => JavaScriptSyntax {
            jsx_element: Some("jsx_element"),
            ..common
        },
        JavaScriptDialect::TypeScript => JavaScriptSyntax {
            type_arguments: Some("type_arguments"),
            type_parameters: Some("type_parameters"),
            type_assertion: Some("type_assertion"),
            ..common
        },
        JavaScriptDialect::Tsx => JavaScriptSyntax {
            type_arguments: Some("type_arguments"),
            type_parameters: Some("type_parameters"),
            jsx_element: Some("jsx_element"),
            ..common
        },
    }
}

#[cfg(test)]
mod tests {
    use tree_sitter::Node;

    use crate::languages::javascript::dialect::{JavaScriptDialect, config_for_dialect};
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

    fn assert_common_grammar_contract_for_dialect(dialect: JavaScriptDialect) {
        let source = r#"
function check(a, b, cond) {
    var x = a + b;
    x += 1;
    var y = !cond;
    var z = callMe(a, b, x);

    if (x > 0) {
        return z;
    }

    while (cond) {
        break;
        continue;
    }

    for (var k in z) {
        callMe(k);
    }

    for (var i = 0; i < 2; i++) {
        callMe(i);
    }

    do {
        callMe(x);
    } while (cond);
}
"#;

        let dialect_config = config_for_dialect(dialect);
        let syntax = syntax_for_dialect(dialect);
        let tree = parse_source(source, dialect_config.parser_language())
            .expect("JavaScript-family parser should parse grammar guard source");
        let root = tree.root_node();

        let if_stmt = first_node_of_kind(root, syntax.if_statement)
            .expect("expected if_statement node in grammar guard source");
        assert!(
            if_stmt
                .child_by_field_name(syntax.condition_field)
                .is_some(),
            "if_statement must expose condition field '{}'",
            syntax.condition_field
        );

        let while_stmt = first_node_of_kind(root, syntax.while_statement)
            .expect("expected while_statement node in grammar guard source");
        assert!(
            while_stmt
                .child_by_field_name(syntax.condition_field)
                .is_some(),
            "while_statement must expose condition field '{}'",
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

        let unary_expr = first_node_of_kind(root, syntax.unary_expression)
            .expect("expected unary_expression node in grammar guard source");
        assert!(
            unary_expr
                .child_by_field_name(syntax.operator_field)
                .is_some(),
            "unary_expression must expose operator field '{}'",
            syntax.operator_field
        );
        assert!(
            unary_expr
                .child_by_field_name(syntax.argument_field)
                .is_some(),
            "unary_expression must expose argument field '{}'",
            syntax.argument_field
        );

        for kind in [
            syntax.binary_expression,
            syntax.augmented_assignment_expression,
            syntax.expression_statement,
            syntax.return_statement,
            syntax.variable_declaration,
            syntax.break_statement,
            syntax.continue_statement,
            syntax.for_statement,
            syntax.for_in_statement,
            syntax.do_statement,
        ] {
            first_node_of_kind(root, kind)
                .unwrap_or_else(|| panic!("expected {kind} node in grammar guard source"));
        }
    }

    #[test]
    fn common_grammar_contract_holds_for_javascript_dialect() {
        assert_common_grammar_contract_for_dialect(JavaScriptDialect::JavaScript);
    }

    #[test]
    fn common_grammar_contract_holds_for_jsx_dialect() {
        assert_common_grammar_contract_for_dialect(JavaScriptDialect::Jsx);
    }

    #[test]
    fn common_grammar_contract_holds_for_typescript_dialect() {
        assert_common_grammar_contract_for_dialect(JavaScriptDialect::TypeScript);
    }

    #[test]
    fn common_grammar_contract_holds_for_tsx_dialect() {
        assert_common_grammar_contract_for_dialect(JavaScriptDialect::Tsx);
    }

    #[test]
    fn dialect_syntax_records_known_grammar_differences() {
        let js = syntax_for_dialect(JavaScriptDialect::JavaScript);
        let ts = syntax_for_dialect(JavaScriptDialect::TypeScript);
        let tsx = syntax_for_dialect(JavaScriptDialect::Tsx);

        assert!(js.type_parameters.is_none());
        assert_eq!(ts.type_parameters, Some("type_parameters"));
        assert_eq!(tsx.type_parameters, Some("type_parameters"));

        assert_eq!(ts.type_assertion, Some("type_assertion"));
        assert!(tsx.type_assertion.is_none());

        assert_eq!(js.jsx_element, Some("jsx_element"));
        assert!(ts.jsx_element.is_none());
        assert_eq!(tsx.jsx_element, Some("jsx_element"));
    }
}
