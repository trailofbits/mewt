use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn sos_swaps_shift_operators() {
    let source = r#"
package main

func shift(x int) int {
    return (x << 1) + (x >> 1)
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "SOS")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "<<" && m.new_text == ">>"),
        "expected SOS to mutate << to >>: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == ">>" && m.new_text == "<<"),
        "expected SOS to mutate >> to <<: {mutants:?}"
    );
}
