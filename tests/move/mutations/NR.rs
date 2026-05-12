use crate::r#move::shared::mutants_for_slug;

#[test]
fn nr_removes_simple_negation() {
    let source = r#"module test::m {
    fun check(ok: bool): bool {
        !ok
    }
}"#;

    let mutants = mutants_for_slug(source, "Move/sui", "NR");
    assert_eq!(
        mutants.len(),
        1,
        "expected exactly one NR mutant removing !ok: {mutants:?}"
    );
    assert_eq!(mutants[0].old_text, "!ok");
    assert_eq!(mutants[0].new_text, "ok");
}

#[test]
fn nr_preserves_parenthesized_operands() {
    let source = r#"module test::m {
    fun check(a: bool, b: bool): bool {
        !(a && b)
    }
}"#;

    let mutants = mutants_for_slug(source, "Move/sui", "NR");
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "!(a && b)" && m.new_text == "(a && b)"),
        "expected NR mutant removing negation but retaining parentheses: {mutants:?}"
    );
}

#[test]
fn nr_ignores_negations_in_comments() {
    let source = r#"module test::m {
    // !ready
    fun noop(): bool {
        true
    }
}"#;

    let mutants = mutants_for_slug(source, "Move/sui", "NR");
    assert!(
        mutants.is_empty(),
        "NR should ignore negations that appear only in comments: {mutants:?}"
    );
}
