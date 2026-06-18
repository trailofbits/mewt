use crate::r#move::shared::assert_only_slug_and_expected_new_texts;

#[test]
fn cos_mutates_comparison_operators() {
    let source = r#"module test::m {
    fun greater(a: u64, b: u64): bool {
        a > b
    }
}"#;

    assert_only_slug_and_expected_new_texts(
        source,
        "move/sui",
        "COS",
        &["==", "!=", "<", "<=", ">="],
    );
}
