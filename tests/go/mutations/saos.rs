use crate::go::integration_tests::assert_slug_has_no_mutants;

#[test]
fn saos_is_not_generated_for_go() {
    let source = r#"
package main
func f(a int) int {
    a <<= 1
    return a
}
"#;

    assert_slug_has_no_mutants(source, "SAOS");
}
