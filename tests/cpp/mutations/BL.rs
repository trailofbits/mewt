use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "BL")
        .collect()
}

#[test]
fn test_bl_replacement_content() {
    let source = r#"
bool f() {
    return true;
}
"#;
    let bl = slug_mutants(source);
    assert_eq!(
        bl.len(),
        1,
        "BL should produce 1 replacement for true: {bl:?}"
    );
    assert_eq!(bl[0].old_text, "true");
    assert_eq!(bl[0].new_text, "false");
}

#[test]
fn test_nullptr_not_treated_as_boolean() {
    let source = r#"
void f() {
    int* p = nullptr;
}
"#;
    let bl = slug_mutants(source);
    assert!(
        bl.is_empty(),
        "BL should not treat nullptr as a boolean literal: {bl:?}"
    );
}
