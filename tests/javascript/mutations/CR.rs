use crate::javascript::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn cr_wraps_statements_in_block_comments() {
    let source = r#"
function maybe(value) {
  if (value) {
    return handle(value);
  }
  return undefined;
}
"#;
    let (_tmp, target) = create_test_target(source, "test.js");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "CR")
        .collect::<Vec<_>>();

    assert!(!mutants.is_empty(), "expected CR mutants");
    for mutant in mutants {
        let trimmed = mutant.new_text.trim();
        assert!(
            trimmed.starts_with("/*") && trimmed.ends_with("*/"),
            "CR mutant should wrap statement in block comment: {:?}",
            trimmed
        );
    }
}

#[test]
fn cr_wraps_typescript_return_statements() {
    let source = r#"
export function answer(flag: boolean): number {
  if (flag) {
    return 42;
  }
  return 0;
}
"#;
    let (_tmp, target) = create_test_target(source, "test.ts");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "CR")
        .collect::<Vec<_>>();

    assert!(
        mutants.iter().any(|m| m.old_text.contains("return 42")),
        "expected CR to wrap TypeScript return statement"
    );
    assert!(
        mutants
            .iter()
            .filter(|m| m.old_text.contains("return 42"))
            .all(|m| m.new_text.trim().starts_with("/*") && m.new_text.trim().ends_with("*/")),
        "CR should wrap TypeScript return statements in comments"
    );
}
