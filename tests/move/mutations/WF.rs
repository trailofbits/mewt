use crate::r#move::shared::assert_only_slug_and_expected_new_texts;

#[test]
fn wf_hardcodes_while_conditions_to_false() {
    let source = r#"module test::m {
    fun count(n: u64): u64 {
        let mut i = 0;
        while (i < n) {
            i = i + 1;
        };
        i
    }
}"#;

    assert_only_slug_and_expected_new_texts(source, "move/sui", "WF", &["false"]);
}
