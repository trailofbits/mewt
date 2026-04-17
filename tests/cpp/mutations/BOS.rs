use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "BOS")
        .collect()
}

#[test]
fn test_bos_replacement_content() {
    let source = r#"
int f(int a, int b) {
    return a & b;
}
"#;
    let bos = slug_mutants(source);
    assert_eq!(
        bos.len(),
        2,
        "BOS should produce 2 replacements for &: {bos:?}"
    );
    assert!(
        bos.iter().all(|m| m.old_text == "&"),
        "All should replace &"
    );
    let new_ops: std::collections::HashSet<_> = bos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["|", "^"].into_iter().collect(),
        "Should replace & with | and ^"
    );
}
