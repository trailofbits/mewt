use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn if_rewrites_conditions_to_false() {
    let source = r#"
fn check(flag: bool) {
    if flag {
        log::info!("true branch");
    }
    if (flag && true) {
        log::info!("parenthesized");
    }
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IF")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected IF mutants to rewrite conditions"
    );

    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    for expected in ["false", "(false)"] {
        assert!(
            replacements.contains(expected),
            "expected IF mutant with new text `{expected}`; replacements: {replacements:?}"
        );
    }
}
