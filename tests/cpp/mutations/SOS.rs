use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "SOS")
        .collect()
}

#[test]
fn test_sos_replacement_content() {
    let source = r#"
int f(int x) {
    return x << 2;
}
"#;
    let sos = slug_mutants(source);
    assert_eq!(
        sos.len(),
        1,
        "SOS should produce 1 replacement for <<: {sos:?}"
    );
    assert_eq!(sos[0].old_text, "<<");
    assert_eq!(sos[0].new_text, ">>");
}
