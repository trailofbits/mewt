use crate::cpp::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn test_sos_replacement_content() {
    let source = r#"
int f(int x) {
    return x << 2;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "SOS", &[">>"]);
}
