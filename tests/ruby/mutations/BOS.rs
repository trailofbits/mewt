use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bos_mutates_bitwise_operators() {
    let source = r#"
result = a & b
"#;
    assert_only_slug_and_expected_new_texts(source, "BOS", &["|", "^"]);
}
