use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn nr_removes_negation_operator() {
    let source = r#"
if !done
  work()
end
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "NR");

    assert!(
        !mutants.is_empty(),
        "NR should produce at least one mutant for !expr"
    );
    assert!(
        mutants.iter().any(|m| m.new_text == "done"),
        "NR should remove the ! operator leaving just the operand: {mutants:?}"
    );
}

#[test]
fn nr_removes_not_keyword() {
    let source = r#"
if not done
  work()
end
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "NR");

    assert!(
        mutants.iter().any(|m| m.new_text == "done"),
        "NR should remove the not keyword leaving just the operand: {mutants:?}"
    );
}
