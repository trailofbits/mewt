use crate::cpp::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn test_baos_replacement_content() {
    let source = r#"
void f() {
    int x = 0xff;
    x &= 0x0f;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BAOS", &["|=", "^="]);
}
