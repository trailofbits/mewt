use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn los_mutates_logical_operators() {
    let source = r#"
fn check(a: bool, b: bool) -> bool {
    if a && b {
        return true;
    }
    a || b
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "LOS")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected LOS mutants for logical operators"
    );

    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    for expected in ["&&", "||"] {
        assert!(
            replacements.contains(expected),
            "expected LOS mutant producing `{expected}`; replacements: {replacements:?}"
        );
    }
}
