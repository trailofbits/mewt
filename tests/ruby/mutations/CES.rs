use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn ces_swaps_case_equality() {
    let source = "a === b; c == d";
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "CES");

    assert_eq!(mutants.len(), 2);
    let mut actual = mutants.into_iter().map(|m| m.new_text).collect::<Vec<_>>();
    actual.sort();
    assert_eq!(actual, vec!["==", "==="]);
}
