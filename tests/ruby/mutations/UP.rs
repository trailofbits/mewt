use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn up_removes_pin_operator() {
    let source = r#"
case val
in ^var
  true
end
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "UP");

    assert!(
        !mutants.is_empty(),
        "UP should produce at least one mutant for pin operator"
    );

    let first = &mutants[0].new_text;
    assert_eq!(first, "var");
}
