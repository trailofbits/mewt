use crate::cpp::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "IT")
}

#[test]
fn it_replaces_condition_with_true() {
    let source = r#"
int f(int x) {
    if (x > 0) {
        return 1;
    }
    return 0;
}
"#;
    let it = slug_mutants(source);
    assert_eq!(it.len(), 1, "Should produce 1 IT mutation: {it:?}");
    assert!(
        it[0].new_text.contains("true"),
        "IT should replace condition with true: {:?}",
        it[0].new_text
    );
}

#[test]
fn it_does_not_target_while_or_for() {
    let source = r#"
void f() {
    while (true) { break; }
    for (int i = 0; i < 10; i++) {}
}
"#;
    let it = slug_mutants(source);
    assert!(
        it.is_empty(),
        "IT should only target if statements, not while/for: {it:?}"
    );
}
