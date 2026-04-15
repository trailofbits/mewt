use crate::go::integration_tests::assert_slug_has_no_mutants;

#[test]
fn baos_is_not_generated_for_go() {
    let source = r#"
package main
func f(a int, b int) int {
    a &= b
    return a
}
"#;

    assert_slug_has_no_mutants(source, "BAOS");
}
