use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

#[test]
fn test_if_it_replacement_content() {
    let source = r#"
int f(int x) {
    if (x > 0) {
        return 1;
    }
    return 0;
}
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let if_mut: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "IF").collect();
    let it_mut: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "IT").collect();

    assert_eq!(if_mut.len(), 1, "Should produce 1 IF mutation: {if_mut:?}");
    assert_eq!(it_mut.len(), 1, "Should produce 1 IT mutation: {it_mut:?}");
    assert!(
        if_mut[0].new_text.contains("false"),
        "IF should replace condition with false: {:?}",
        if_mut[0].new_text
    );
    assert!(
        it_mut[0].new_text.contains("true"),
        "IT should replace condition with true: {:?}",
        it_mut[0].new_text
    );
}
