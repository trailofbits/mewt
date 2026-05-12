use crate::r#move::shared::assert_only_slug_and_expected_new_texts;

#[test]
fn lc_swaps_break_and_continue() {
    let source = r#"module test::m {
    fun loopy(mut n: u64): u64 {
        while (n > 0) {
            if (n > 10) { break };
            continue;
        };
        n
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "Move/sui", "LC", &["continue", "break"]);
}
