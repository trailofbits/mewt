use crate::go::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn as_mutation_swaps_adjacent_arguments() {
    let source = r#"
package main

func call(a, b, c int) int {
    return compute(a, b, c)
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AS", &["b, a", "c, b"]);
}
