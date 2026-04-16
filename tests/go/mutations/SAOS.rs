use crate::go::integration_tests::{assert_only_slug_and_expected_new_texts, create_test_target};
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

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

#[test]
fn saos_swaps_right_shift_assignment_to_left() {
    let source = r#"
package main
func g(a int) int {
    a >>= 1
    return a
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "SAOS")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == ">>=" && m.new_text == "<<="),
        "expected SAOS to turn >>= into <<=: {mutants:?}"
    );
}
