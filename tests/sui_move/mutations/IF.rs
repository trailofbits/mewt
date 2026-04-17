use crate::sui_move::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn if_hardcodes_if_conditions_to_false() {
    let source = r#"module test::m {
    fun check(x: u64): bool {
        if (x > 0) { true } else { false }
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "IF", &["false"]);
}
