use crate::go::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn aos_mutates_arithmetic_operators() {
    let source = r#"
package main

func calc(a, b int) int {
    return a + b
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AOS", &["-", "*", "/", "%"]);
}
