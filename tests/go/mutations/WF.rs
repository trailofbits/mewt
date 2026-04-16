use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn wf_mutation_is_not_generated_in_go() {
    let source = r#"
package main

func f(x int) int {
    if x > 0 {
        return x
    }
    return 0
}
"#;

    let (_tmp, target) = create_test_target(source);
    let engine = GoLanguageEngine::new();
    let wf_mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "WF")
        .collect();

    assert!(
        wf_mutants.is_empty(),
        "Go should not produce WF mutants, found: {wf_mutants:?}"
    );
}
