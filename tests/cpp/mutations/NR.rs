use crate::cpp::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "NR")
}

#[test]
fn test_negation_removal_ignores_other_unary_ops() {
    let source = r#"
int f(int x) {
    return -x;
}
"#;
    let nr = slug_mutants(source);
    assert!(
        nr.is_empty(),
        "NR should not trigger on - unary operator: {nr:?}"
    );
}

#[test]
fn test_negation_removal_complex_expression() {
    let source = r#"
bool check(bool a, bool b) {
    return !(a && b);
}
"#;
    let nr = slug_mutants(source);
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(a && b)" && m.new_text == "(a && b)"),
        "NR should remove negation preserving parenthesized operand: {nr:?}"
    );
}

#[test]
fn test_negation_removal_in_comment_ignored() {
    let source = r#"
// if (!flag) { return; }
/* !x */
int main() { return 0; }
"#;
    let nr = slug_mutants(source);
    assert!(
        nr.is_empty(),
        "NR should not generate mutations inside comments: {nr:?}"
    );
}
