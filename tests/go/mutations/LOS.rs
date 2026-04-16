use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn los_swaps_logical_or_and_and() {
    let source = r#"
package main

func check(a, b, c, d bool) bool {
    if a && b {
        return true
    }
    if c || d {
        return false
    }
    return a
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "LOS")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "&&" && m.new_text == "||"),
        "expected LOS to mutate && to ||: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "||" && m.new_text == "&&"),
        "expected LOS to mutate || to &&: {mutants:?}"
    );
}
