use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;
use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn aos_mutates_arithmetic_operators() {
    let source = r#"
result = a + b
"#;
    assert_only_slug_and_expected_new_texts(source, "AOS", &["-", "*", "/", "%", "**"]);
}

#[test]
fn aos_mutates_modulo_operator() {
    let source = r#"
is_even = n % 2
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "AOS");
    assert!(
        !mutants.is_empty(),
        "AOS should produce mutants for the % operator"
    );
    // new_text is the replacement operator token
    let new_texts: Vec<&str> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    assert!(
        new_texts.contains(&"+"),
        "AOS should shuffle % to + (among others), got {new_texts:?}"
    );
    assert!(
        !new_texts.contains(&"%"),
        "AOS should not produce identity mutation for %"
    );
}

#[test]
fn aos_mutates_exponentiation_operator() {
    let source = r#"
scaled = 10 ** precision
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "AOS");
    assert!(
        !mutants.is_empty(),
        "AOS should produce mutants for the ** operator"
    );
    let new_texts: Vec<&str> = mutants.iter().map(|m| m.new_text.as_str()).collect();
    assert!(
        new_texts.contains(&"+"),
        "AOS should shuffle ** to + (among others), got {new_texts:?}"
    );
    assert!(
        !new_texts.contains(&"**"),
        "AOS should not produce identity mutation for **"
    );
}
