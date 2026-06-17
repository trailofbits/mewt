use crate::r#move::shared::assert_only_slug_and_expected_new_texts;

#[test]
fn cr_wraps_statements_in_comments() {
    let source = r#"module test::m {
    fun maybe_add(a: u64, b: u64): u64 {
        let sum = a + b;
        sum
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "move/sui", "CR", &["/* ", " */"]);
}
