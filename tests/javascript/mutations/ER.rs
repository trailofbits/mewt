use crate::javascript::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn er_replaces_statements_with_throw() {
    let source = r#"
function maybeAdd(x) {
  if (x > 0) {
    return x + 1;
  }
  return x - 1;
}
"#;
    let (_tmp, target) = create_test_target(source, "test.js");
    let mutants: Vec<_> = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "ER")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected ER mutants to replace statements"
    );
    for mutant in mutants {
        assert_eq!(
            mutant.new_text.trim(),
            "throw new Error(\"mewt\");",
            "ER should replace statement with a throw expression"
        );
    }
}

#[test]
fn er_covers_typescript_variable_statements() {
    let source = r#"
export function configure(flag: boolean) {
  let retries = 0;
  if (flag) {
    retries = 3;
  }
  return retries;
}
"#;
    let (_tmp, target) = create_test_target(source, "test.ts");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "ER")
        .collect::<Vec<_>>();

    assert!(
        mutants.iter().any(|m| m.old_text.contains("retries = 3")),
        "expected ER to target TypeScript assignment statements"
    );
}
