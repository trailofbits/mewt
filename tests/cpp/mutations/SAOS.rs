use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "SAOS")
        .collect()
}

#[test]
fn test_saos_replacement_content() {
    let source = r#"
void f() {
    int x = 1;
    x <<= 3;
}
"#;
    let saos = slug_mutants(source);
    assert_eq!(
        saos.len(),
        1,
        "SAOS should produce 1 replacement for <<=: {saos:?}"
    );
    assert_eq!(saos[0].old_text, "<<=");
    assert_eq!(saos[0].new_text, ">>=");
}
