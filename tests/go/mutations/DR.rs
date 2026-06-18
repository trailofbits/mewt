use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn dr_replaces_defer_statement_with_deferred_call() {
    let source = r#"
package main

func cleanup(close func()) {
    defer close()
}
"#;

    let (_tmp, target) = create_test_target(source);
    let engine = GoLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "DR")
        .collect();

    assert_eq!(mutants.len(), 1, "expected one DR mutant: {mutants:?}");
    assert_eq!(mutants[0].old_text.trim(), "defer close()");
    assert_eq!(mutants[0].new_text.trim(), "close()");
}
