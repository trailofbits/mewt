use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn snr_removes_safe_navigation() {
    let source = "foo&.bar";
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "SNR");

    assert_eq!(mutants.len(), 1);
    assert_eq!(mutants[0].new_text, ".");
}
