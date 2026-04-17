use crate::cpp::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn test_saos_replacement_content() {
    let source = r#"
void f() {
    int x = 1;
    x <<= 3;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "SAOS", &[">>="]);
}
