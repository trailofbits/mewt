use crate::go::integration_tests::{assert_only_slug_and_expected_new_texts, create_test_target};
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn bos_mutates_bitwise_operators() {
    let source = r#"
package main

func combine(a, b int) int {
    return a & b
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BOS", &["|", "^", "&^"]);
}

#[test]
fn bos_handles_bit_clear_operator() {
    let source = r#"
package main

func clear(a, b int) int {
    return a &^ b
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "BOS")
        .collect::<Vec<_>>();

    assert!(
        mutants.iter().any(|m| m.old_text == "&^"),
        "expected BOS to target the Go-specific &^ operator: {mutants:?}"
    );
}
