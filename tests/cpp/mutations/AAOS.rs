use crate::cpp::integration_tests::{assert_only_slug_and_expected_new_texts, create_test_target};
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;

#[test]
fn test_aaos_replacement_content() {
    let source = r#"
void f() {
    int x = 10;
    x += 5;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AAOS", &["-=", "*=", "/=", "%="]);
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
