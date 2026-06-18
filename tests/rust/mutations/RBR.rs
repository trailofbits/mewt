use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn rbr_swaps_exclusive_and_inclusive_range_bounds() {
    let source = r#"
fn ranges(end: usize) {
    for _ in 0..end {}
    for _ in 0..=end {}
}
"#;

    let (_tmp, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "RBR")
        .collect();

    assert!(!mutants.is_empty(), "expected RBR mutants");
    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    assert!(
        replacements.contains(".."),
        "missing .. replacement: {replacements:?}"
    );
    assert!(
        replacements.contains("..="),
        "missing ..= replacement: {replacements:?}"
    );
}
