use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn baos_mutates_bitwise_assignment_operators() {
    let source = r#"
x &= mask
"#;
    assert_only_slug_and_expected_new_texts(source, "BAOS", &["|=", "^="]);
}
