use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "CR")
        .collect()
}

#[test]
fn test_comment_replacement() {
    let source = r#"
int f() {
    int x = 42;
    return x;
}
"#;
    let cr = slug_mutants(source);
    assert!(!cr.is_empty(), "Should generate CR mutations");
    for m in &cr {
        assert!(
            m.new_text.starts_with("/*") && m.new_text.ends_with("*/"),
            "CR should wrap in block comments: {}",
            m.new_text
        );
    }
}

#[test]
fn cr_does_not_produce_nested_comments() {
    let source = r#"
// int x = 1;
/* int y = 2; */
int main() {
    return 0;
}
"#;
    let cr = slug_mutants(source);
    for m in &cr {
        assert!(
            !m.new_text.contains("/* /*") && !m.new_text.contains("*/ */"),
            "CR should not produce nested block comments: {:?}",
            m.new_text
        );
    }
}
