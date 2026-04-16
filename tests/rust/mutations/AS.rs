use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn as_swaps_adjacent_arguments() {
    let source = r#"
fn call_all() {
    consume(foo(1, 2, 3));
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "AS")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected AS mutants to swap adjacent arguments"
    );

    let new_texts: HashSet<_> = mutants
        .iter()
        .map(|m| m.new_text.trim().to_string())
        .collect();
    for expected in ["2, 1", "3, 2"] {
        assert!(
            new_texts.contains(expected),
            "expected swapped argument text `{expected}`; new_texts: {new_texts:?}"
        );
    }
}
