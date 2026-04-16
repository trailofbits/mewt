use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn er_replaces_statements_with_assertions() {
    let source = r#"
fn maybe_add(x: i32) -> i32 {
    if x > 0 {
        return x + 1;
    }
    x - 1
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "ER")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected ER mutants to replace statements"
    );

    for mutant in mutants {
        assert_eq!(mutant.new_text, "assert!(false);");
    }
}
