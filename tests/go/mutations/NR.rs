use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;
use mewt::types::Mutant;

fn nr_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "NR")
        .collect()
}

#[test]
fn nr_removes_simple_negation() {
    let source = r#"
package main

func check(ok bool) bool {
    if !ok {
        return false
    }
    return ok
}
"#;

    let nr = nr_mutants(source);
    assert_eq!(nr.len(), 1, "expected exactly one NR mutant: {nr:?}");
    assert_eq!(nr[0].old_text, "!ok");
    assert_eq!(nr[0].new_text, "ok");
}

#[test]
fn nr_preserves_parenthesized_operands() {
    let source = r#"
package main

func check(a, b bool) bool {
    return !(a && b)
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
package main

func value(x int) int {
    return -x
}
"#;

    let nr = nr_mutants(source);
    assert!(
        nr.is_empty(),
        "NR should not trigger on unary minus: {nr:?}"
    );
}

#[test]
fn nr_ignores_negations_in_comments() {
    let source = r#"
package main

// if !ready { panic("nope") }
/* !flag */
func noop() {}
"#;

    let nr = nr_mutants(source);
    assert!(
        nr.is_empty(),
        "NR should ignore negations that appear only in comments: {nr:?}"
    );
}
