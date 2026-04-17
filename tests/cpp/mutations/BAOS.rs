use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "BAOS")
        .collect()
}

#[test]
fn test_baos_replacement_content() {
    let source = r#"
void f() {
    int x = 0xff;
    x &= 0x0f;
}
"#;
    let baos = slug_mutants(source);
    assert_eq!(
        baos.len(),
        2,
        "BAOS should produce 2 replacements for &=: {baos:?}"
    );
    assert!(
        baos.iter().all(|m| m.old_text == "&="),
        "All should replace &="
    );
    let new_ops: std::collections::HashSet<_> = baos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["|=", "^="].into_iter().collect(),
        "Should replace &= with |= and ^="
    );
}
