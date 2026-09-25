use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn rbr_swaps_exclusive_and_inclusive_range_bounds() {
    let source = r#"
1..10
1...10
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "RBR");

    assert!(
        mutants.len() >= 2,
        "RBR should produce at least two mutants for ranges"
    );

    let first = &mutants[0].new_text;
    let second = &mutants[1].new_text;

    assert!(first.contains("..."));
    assert!(second.contains(".."));
}
