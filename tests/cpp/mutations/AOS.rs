use crate::cpp::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn test_aos_replacement_content() {
    let source = r#"
int f(int a, int b) {
    return a + b;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AOS", &["-", "*", "/", "%"]);
}
