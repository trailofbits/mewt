use crate::cpp::integration_tests::{create_test_target, mutants_for_slug};
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "COS")
}

#[test]
fn test_cos_replacement_content() {
    let source = r#"
bool f(int a, int b) {
    return a == b;
}
"#;
    let cos = slug_mutants(source);
    assert_eq!(
        cos.len(),
        5,
        "COS should produce 5 replacements for ==: {cos:?}"
    );
    assert!(
        cos.iter().all(|m| m.old_text == "=="),
        "All should replace =="
    );
    let new_ops: std::collections::HashSet<_> = cos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["!=", "<", "<=", ">", ">="].into_iter().collect(),
        "Should replace == with all other comparison operators"
    );
}

#[test]
fn test_operator_overload_not_mutated_in_signature() {
    let source = r#"
struct Foo {
    int val;
    bool operator==(const Foo& other) {
        return val == other.val;
    }
};
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // COS should only mutate the `==` inside the body (val == other.val),
    // not the `==` in the operator signature (operator==)
    let cos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    for m in &cos {
        assert!(
            !m.old_text.contains("operator"),
            "COS should not mutate operator== in the signature: {:?}",
            m.old_text
        );
    }
}

#[test]
fn test_ternary_operator() {
    let source = r#"
int max_val(int a, int b) {
    return a > b ? a : b;
}
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // COS should mutate the > inside the ternary condition
    let cos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    assert!(
        cos.iter().any(|m| m.old_text == ">"),
        "COS should mutate the comparison in a ternary expression: {cos:?}"
    );
}
