use crate::cpp::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn test_los_replacement_content() {
    let source = r#"
bool f(bool a, bool b) {
    return a && b;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "LOS", &["||"]);
}
