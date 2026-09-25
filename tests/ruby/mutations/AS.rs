use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn as_swaps_call_arguments() {
    let source = r#"
foo(a, b)
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "AS");

    assert!(
        !mutants.is_empty(),
        "AS should produce at least one mutant for a two-argument call"
    );
}
