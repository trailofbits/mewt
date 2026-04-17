use crate::cpp::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "WF")
}

#[test]
fn test_wf_replacement_content() {
    let source = r#"
void f() {
    while (true) {
        break;
    }
}
"#;
    let wf = slug_mutants(source);
    assert_eq!(
        wf.len(),
        1,
        "WF should produce 1 mutation for while condition: {wf:?}"
    );
    assert!(
        wf[0].new_text.contains("false"),
        "WF should replace condition with false: {:?}",
        wf[0].new_text
    );
}

#[test]
fn wf_targets_do_while_condition() {
    let source = r#"
int sum_to(int n) {
    int total = 0;
    int i = 1;
    do {
        total += i;
        i++;
    } while (i <= n);
    return total;
}
"#;
    let wf = slug_mutants(source);
    assert_eq!(
        wf.len(),
        1,
        "WF should produce 1 mutation for do-while condition: {wf:?}"
    );
    assert!(
        wf[0].new_text.contains("false"),
        "WF should replace do-while condition with false: {:?}",
        wf[0].new_text
    );
}

#[test]
fn wf_does_not_target_for_conditions() {
    let source = r#"
void f() {
    for (int i = 0; i < 10; i++) {}
}
"#;
    let wf = slug_mutants(source);
    assert!(
        wf.is_empty(),
        "WF should not target for-loop conditions: {wf:?}"
    );
}
