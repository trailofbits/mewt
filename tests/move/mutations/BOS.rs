use crate::r#move::shared::assert_only_slug_and_expected_new_texts;

#[test]
fn bos_mutates_bitwise_operators() {
    let source = r#"module test::m {
    fun mask(a: u64, b: u64): u64 {
        a & b
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "move/sui", "BOS", &["|", "^"]);
}
