use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn if_mutation_forces_condition_to_false() {
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
        .filter(|m| m.mutation_slug == "IF")
        .collect::<Vec<_>>();

    assert!(
        !mutants.is_empty(),
        "expected IF to mutate the condition to false"
    );
    assert!(
        mutants
            .iter()
            .all(|m| m.new_text == "(false)" || m.new_text == "false"),
        "unexpected IF replacements: {mutants:?}"
    );
}

#[test]
fn if_mutation_preserves_parentheses() {
    let source = r#"
package main

func guarded(ok bool) {
    if (ok && ready()) {
        action()
    }
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IF")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "(ok && ready())" && m.new_text == "(false)"),
        "expected IF mutant to retain parentheses around the replacement: {mutants:?}"
    );
}
