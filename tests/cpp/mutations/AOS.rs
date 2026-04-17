use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "AOS")
        .collect()
}

#[test]
fn test_aos_replacement_content() {
    let source = r#"
int f(int a, int b) {
    return a + b;
}
"#;
    let aos = slug_mutants(source);
    // a + b should produce 4 replacements: -, *, /, %
    assert_eq!(
        aos.len(),
        4,
        "AOS should produce 4 replacements for +: {aos:?}"
    );
    assert!(
        aos.iter().all(|m| m.old_text == "+"),
        "All should replace +"
    );
    let new_ops: std::collections::HashSet<_> = aos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["-", "*", "/", "%"].into_iter().collect(),
        "Should replace + with all other arithmetic operators"
    );
}
