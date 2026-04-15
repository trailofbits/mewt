use std::collections::HashSet;

use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn lc_swaps_loop_control_statements() {
    let source = r#"
fn loop_example(values: &[i32]) {
    for value in values {
        if *value == 0 {
            break;
        }
        if *value == 1 {
            continue;
        }
    }
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "LC")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected LC mutants to swap loop control keywords"
    );

    let replacements: HashSet<_> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    for expected in ["break", "continue"] {
        assert!(
            replacements.contains(expected),
            "expected LC mutant replacing with `{expected}`; replacements: {replacements:?}"
        );
    }
}
