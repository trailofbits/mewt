use crate::go::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn cos_mutates_comparison_operators() {
    let source = r#"
package main

func cmp(a, b int) bool {
    return a == b
}
"#;

    assert_only_slug_and_expected_new_texts(source, "COS", &["!=", "<", "<=", ">", ">="]);
}
