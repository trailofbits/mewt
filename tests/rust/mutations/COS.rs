use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn cos_mutates_comparison_operators() {
    let source = r#"
fn cmp(a: i32, b: i32) -> bool {
    a == b
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected COS mutants for comparison expressions"
    );

    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    for expected in ["!=", "<", "<=", ">", ">="] {
        assert!(
            replacements.contains(expected),
            "expected COS mutant producing operator `{expected}`; replacements: {replacements:?}"
        );
    }
}
