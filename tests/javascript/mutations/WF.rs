use crate::javascript::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn wf_forces_while_condition_to_false() {
    let source = r#"
function poll(queue) {
  while (queue.length > 0) {
    queue.shift();
  }
}
"#;
    let (_tmp, target) = create_test_target(source, "test.js");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "WF")
        .collect::<Vec<_>>();

    let replacements: Vec<String> = mutants
        .iter()
        .map(|m| m.new_text.trim().to_string())
        .collect();
    assert!(
        replacements
            .iter()
            .any(|text| text == "false" || text == "(false)"),
        "expected WF to replace while condition with false; found {replacements:?}"
    );
}

#[test]
fn wf_preserves_parentheses_in_ts() {
    let source = r#"
export function drain(queue: string[]) {
  while (queue.length && queue.shift()) {
    continue;
  }
}
"#;
    let (_tmp, target) = create_test_target(source, "test.ts");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "WF")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "(queue.length && queue.shift())"
                && m.new_text == "(false)"),
        "expected WF mutant to retain parentheses around false"
    );
}
