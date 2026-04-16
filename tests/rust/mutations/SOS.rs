use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn sos_mutates_shift_operators() {
    let source = r#"
fn shifts(x: u32) -> (u32, u32) {
    (x << 1, x >> 1)
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "SOS")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected SOS mutants for shift operators"
    );

    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    for expected in ["<<", ">>"] {
        assert!(
            replacements.contains(expected),
            "expected SOS mutant producing `{expected}`; replacements: {replacements:?}"
        );
    }
}
