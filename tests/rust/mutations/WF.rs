use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn wf_rewrites_while_conditions_to_false() {
    let source = r#"
fn drain(mut values: Vec<i32>) {
    while values.pop().is_some() {
        log::debug!("draining");
    }
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "WF")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected WF mutants to rewrite while condition"
    );

    assert!(
        mutants.iter().all(|m| m.new_text == "false"),
        "WF mutants should force condition to false: {mutants:?}"
    );
}
