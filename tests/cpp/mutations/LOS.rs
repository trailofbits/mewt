use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "LOS")
        .collect()
}

#[test]
fn test_los_replacement_content() {
    let source = r#"
bool f(bool a, bool b) {
    return a && b;
}
"#;
    let los = slug_mutants(source);
    assert_eq!(
        los.len(),
        1,
        "LOS should produce 1 replacement for &&: {los:?}"
    );
    assert_eq!(los[0].old_text, "&&");
    assert_eq!(los[0].new_text, "||");
}
