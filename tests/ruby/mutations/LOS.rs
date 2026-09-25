use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn los_mutates_logical_operators() {
    let source = r#"
result = a && b
"#;
    assert_only_slug_and_expected_new_texts(source, "LOS", &["||", "and", "or"]);
}

#[test]
fn los_includes_keyword_operators() {
    let source = r#"
result = a and b
"#;
    assert_only_slug_and_expected_new_texts(source, "LOS", &["&&", "||", "or"]);
}
