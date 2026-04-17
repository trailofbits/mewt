use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "AS")
        .collect()
}

#[test]
fn test_as_replacement_content() {
    let source = r#"
int add(int a, int b) { return a + b; }
int main() {
    return add(10, 20);
}
"#;
    let as_mut = slug_mutants(source);
    assert_eq!(
        as_mut.len(),
        1,
        "AS should produce 1 swap for add(10, 20): {as_mut:?}"
    );
    assert!(
        as_mut[0].old_text.contains("10") && as_mut[0].old_text.contains("20"),
        "AS old_text should contain both args: {:?}",
        as_mut[0].old_text
    );
    // Swapped: 20, 10 instead of 10, 20
    assert!(
        as_mut[0].new_text.starts_with("20") && as_mut[0].new_text.ends_with("10"),
        "AS should swap argument order: {:?}",
        as_mut[0].new_text
    );
}
