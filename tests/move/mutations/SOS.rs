use crate::r#move::shared::assert_only_slug_and_expected_new_texts;

#[test]
fn sos_mutates_shift_operators() {
    let source = r#"module test::m {
    fun shl(a: u64): u64 {
        a << 1
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "move/sui", "SOS", &[">>"]);
}
