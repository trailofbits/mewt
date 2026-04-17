use crate::sui_move::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn as_swaps_adjacent_arguments() {
    let source = r#"module test::m {
    fun call(): u64 {
        helper(1, 2, 3)
    }

    fun helper(a: u64, b: u64, c: u64): u64 {
        a + b + c
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "AS", &["2, 1", "3, 2"]);
}
