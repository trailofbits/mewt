use crate::go::integration_tests::{assert_only_slug_and_expected_new_texts, create_test_target};
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn baos_mutates_bitwise_assignments() {
    let source = r#"
package main
func f(a int, b int) int {
    a &= b
    a &^= b
    return a
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BAOS", &["|=", "^=", "&^=", "&="]);
}

#[test]
fn baos_targets_go_bit_clear_assignment() {
    let source = r#"
package main
func g(a int, b int) int {
    a &^= b
    return a
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "BAOS")
        .collect::<Vec<_>>();

    assert!(
        mutants.iter().any(|m| m.old_text == "&^="),
        "expected BAOS mutant targeting &^=: {mutants:?}"
    );
}
