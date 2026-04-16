use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn cr_wraps_statements_in_block_comments() {
    let source = r#"
fn demo() {
    let value = compute();
    println!("{value}");
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "CR")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected CR mutants to wrap statements"
    );

    for mutant in mutants {
        assert!(
            mutant.new_text.starts_with("/* ") && mutant.new_text.ends_with(" */"),
            "CR mutant should wrap text in block comments: {:?}",
            mutant.new_text
        );
    }
}
