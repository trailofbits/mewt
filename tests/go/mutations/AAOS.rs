use crate::go::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn aaos_mutates_arithmetic_assignments() {
    let source = r#"
package main
func f(a int, b int) int {
    a += b
    return a
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AAOS", &["-=", "*=", "/=", "%="]);
}
