use crate::go::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn saos_mutates_shift_assignments() {
    let source = r#"
package main
func f(a int) int {
    a <<= 1
    return a
}
"#;

    assert_only_slug_and_expected_new_texts(source, "SAOS", &[">>="]);
}
