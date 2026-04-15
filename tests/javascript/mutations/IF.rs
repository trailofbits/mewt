use crate::javascript::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn if_mutation_forces_condition_to_false() {
    let source = r#"
function check(a, b) {
  if (a > b) {
    return true;
  }
  return false;
}
"#;
    let (_tmp, target) = create_test_target(source, "test.js");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IF")
        .collect::<Vec<_>>();

    assert!(
        mutants.iter().any(|m| m.new_text.trim() == "false"),
        "expected IF mutant to replace condition with false"
    );
}

#[test]
fn if_mutation_preserves_parentheses_in_ts() {
    let source = r#"
export function guarded(ok: boolean, ready: () => boolean) {
  if (ok && ready()) {
    action();
  }
}
"#;
    let (_tmp, target) = create_test_target(source, "test.ts");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IF")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "(ok && ready())" && m.new_text == "(false)"),
        "expected IF mutant to retain parentheses around the replacement"
    );
}
