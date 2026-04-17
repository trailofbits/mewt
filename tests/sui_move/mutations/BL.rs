use crate::sui_move::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bl_flips_boolean_literals() {
    let source = r#"module test::m {
    fun ready(): bool {
        true
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "BL", &["false"]);
}
