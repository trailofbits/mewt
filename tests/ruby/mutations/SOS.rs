use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn sos_mutates_shift_operators() {
    let source = r#"
result = a << 2
"#;
    assert_only_slug_and_expected_new_texts(source, "SOS", &[">>"]);
}
