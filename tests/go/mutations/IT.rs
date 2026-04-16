use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn it_mutation_forces_condition_to_true() {
    let source = r#"
package main

func check(a, b int) bool {
    if (a > b) {
        return true
    }
    return false
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IT")
        .collect::<Vec<_>>();

    assert!(
        !mutants.is_empty(),
        "expected IT to mutate the condition to true"
    );
    assert!(
        mutants
            .iter()
            .all(|m| m.new_text == "(true)" || m.new_text == "true"),
        "unexpected IT replacements: {mutants:?}"
    );
}

#[test]
fn it_mutation_preserves_parentheses() {
    let source = r#"
package main

func guarded(ok bool) bool {
    if (ok && ready()) {
        return ok
    }
    return !ok
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IT")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "(ok && ready())" && m.new_text == "(true)"),
        "expected IT mutant to retain parentheses around the replacement: {mutants:?}"
    );
}
