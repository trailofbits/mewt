use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn bl_flips_boolean_literals() {
    let source = r#"
fn main() {
    let ready = true;
    if ready {
        println!("ok");
    }
}
"#;

    let (_tmp_dir, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "BL")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected BL mutants to flip boolean literals"
    );

    assert!(
        mutants.iter().all(|m| m.new_text == "false"),
        "BL mutants should flip `true` to `false`: {mutants:?}"
    );
}
