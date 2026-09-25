use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;
use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn aaos_mutates_arithmetic_assignment_operators() {
    let source = r#"
x += 1
"#;
    assert_only_slug_and_expected_new_texts(source, "AAOS", &["-=", "*=", "/=", "%=", "**="]);
}

#[test]
fn aaos_mutates_modulo_assignment_operator() {
    let source = r#"
x %= 2
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "AAOS");
    assert!(
        !mutants.is_empty(),
        "AAOS should produce mutants for the %= operator"
    );
    // new_text is the replacement operator token
    let new_texts: Vec<&str> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    assert!(
        new_texts.contains(&"+="),
        "AAOS should shuffle %= to += (among others), got {new_texts:?}"
    );
    assert!(
        !new_texts.contains(&"%="),
        "AAOS should not produce identity mutation for %="
    );
}

#[test]
fn aaos_mutates_exponentiation_assignment_operator() {
    let source = r#"
x **= 2
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "AAOS");
    assert!(
        !mutants.is_empty(),
        "AAOS should produce mutants for the **= operator"
    );
    let new_texts: Vec<&str> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    assert!(
        new_texts.contains(&"+="),
        "AAOS should shuffle **= to += (among others), got {new_texts:?}"
    );
    assert!(
        !new_texts.contains(&"**="),
        "AAOS should not produce identity mutation for **="
    );
}
