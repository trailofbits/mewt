use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "AAOS")
        .collect()
}

#[test]
fn test_aaos_replacement_content() {
    let source = r#"
void f() {
    int x = 10;
    x += 5;
}
"#;
    let aaos = slug_mutants(source);
    assert_eq!(
        aaos.len(),
        4,
        "AAOS should produce 4 replacements for +=: {aaos:?}"
    );
    assert!(
        aaos.iter().all(|m| m.old_text == "+="),
        "All should replace +="
    );
    let new_ops: std::collections::HashSet<_> = aaos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["-=", "*=", "/=", "%="].into_iter().collect(),
        "Should replace += with all other arithmetic assignment operators"
    );
}

#[test]
fn test_compound_assignment_mutations() {
    let source = r#"
void f() {
    int x = 0;
    x += 1;
    x -= 2;
    x *= 3;
    x /= 4;
    x &= 0xff;
    x |= 0x01;
    x <<= 2;
    x >>= 1;
}
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();

    assert!(slugs.contains("AAOS"), "Should generate AAOS mutations");
    assert!(slugs.contains("BAOS"), "Should generate BAOS mutations");
    assert!(slugs.contains("SAOS"), "Should generate SAOS mutations");
}
