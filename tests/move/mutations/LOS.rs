use crate::r#move::shared::assert_only_slug_and_expected_new_texts;

#[test]
fn los_mutates_logical_operators() {
    let source = r#"module test::m {
    fun both(a: bool, b: bool): bool {
        a && b
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "move/sui", "LOS", &["||"]);
}
