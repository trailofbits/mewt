use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn it_rewrites_conditions_to_true() {
    let source = r#"
fn check(flag: bool) {
    if flag {
        log::info!("true branch");
    }
    if (flag && false) {
        log::info!("parenthesized");
    }
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IT")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected IT mutants to rewrite conditions"
    );

    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    for expected in ["true", "(true)"] {
        assert!(
            replacements.contains(expected),
            "expected IT mutant with new text `{expected}`; replacements: {replacements:?}"
        );
    }
}
