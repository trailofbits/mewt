use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn bos_mutates_bitwise_operators() {
    let source = r#"
fn flags(a: u8, b: u8) -> u8 {
    a & b
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "BOS")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected BOS mutants for bitwise expressions"
    );

    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    for expected in ["|", "^"] {
        assert!(
            replacements.contains(expected),
            "expected BOS mutant producing operator `{expected}`; replacements: {replacements:?}"
        );
    }
}
