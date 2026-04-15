use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn lc_swaps_break_and_continue() {
    let source = r#"
package main

func sumPositives(xs []int) int {
    total := 0
    for _, x := range xs {
        if x < 0 {
            break
        }
        if x == 0 {
            continue
        }
        total += x
    }
    return total
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "LC")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "break" && m.new_text == "continue"),
        "expected LC to turn break into continue: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "continue" && m.new_text == "break"),
        "expected LC to turn continue into break: {mutants:?}"
    );
}
