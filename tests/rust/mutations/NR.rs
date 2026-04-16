use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;
use mewt::types::Mutant;

fn nr_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp_dir, target) = create_test_target(source);
    RustLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "NR")
        .collect()
}

#[test]
fn nr_removes_simple_negation() {
    let source = r#"
fn main() {
    let x = true;
    if !x {
        println!("negated");
    }
}
"#;

    let nr = nr_mutants(source);
    assert_eq!(nr.len(), 1, "expected exactly one NR mutant");
    assert_eq!(nr[0].old_text, "!x");
    assert_eq!(nr[0].new_text, "x");
}

#[test]
fn nr_preserves_parenthesized_operands() {
    let source = r#"
fn check(a: bool, b: bool) -> bool {
    !(a && b)
}
"#;

    let nr = nr_mutants(source);
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(a && b)" && m.new_text == "(a && b)"),
        "expected NR mutant removing negation but retaining parentheses: {nr:?}"
    );
}

#[test]
fn nr_ignores_non_negation_unary_ops() {
    let source = r#"
fn main() {
    let x = -1;
    let y = *x;
}
"#;

    let nr = nr_mutants(source);
    assert!(nr.is_empty(), "NR should not trigger on - or *: {nr:?}");
}

#[test]
fn nr_skips_negations_in_comments() {
    let source = r#"
fn main() {
    // if !x { }
    /* !flag */
    let ready = true;
}
"#;

    let nr = nr_mutants(source);
    assert!(
        nr.is_empty(),
        "NR should ignore negations that appear only in comments: {nr:?}"
    );
}
