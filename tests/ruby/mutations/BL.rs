use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bl_flips_true_to_false() {
    let source = r#"
x = true
"#;
    assert_only_slug_and_expected_new_texts(source, "BL", &["false"]);
}

#[test]
fn bl_flips_false_to_true() {
    let source = r#"
x = false
"#;
    assert_only_slug_and_expected_new_texts(source, "BL", &["true"]);
}
