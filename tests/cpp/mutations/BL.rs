use crate::cpp::integration_tests::{assert_only_slug_and_expected_new_texts, mutants_for_slug};

#[test]
fn test_bl_replacement_content() {
    let source = r#"
bool f() {
    return true;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BL", &["false"]);
}

#[test]
fn test_nullptr_not_treated_as_boolean() {
    let source = r#"
void f() {
    int* p = nullptr;
}
"#;
    let bl = mutants_for_slug(source, "BL");
    assert!(
        bl.is_empty(),
        "BL should not treat nullptr as a boolean literal: {bl:?}"
    );
}
