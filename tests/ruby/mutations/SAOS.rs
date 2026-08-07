use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn saos_mutates_shift_assignment_operators() {
    let source = r#"
x <<= 2
"#;
    assert_only_slug_and_expected_new_texts(source, "SAOS", &[">>="]);
}
