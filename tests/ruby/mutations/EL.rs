use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn el_empties_literals() {
    let source = "a = [1, 2]; b = { c: 3 }; d = \"foo\"";
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "EL");

    assert_eq!(mutants.len(), 3);
    let mut actual = mutants.into_iter().map(|m| m.new_text).collect::<Vec<_>>();
    actual.sort();
    assert_eq!(actual, vec!["\"\"", "[]", "{}"]);
}
