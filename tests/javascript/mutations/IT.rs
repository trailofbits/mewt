use crate::javascript::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn it_mutation_forces_condition_to_true() {
    let source = r#"
function shouldRetry(errorCount) {
  if (errorCount === 0) {
    return false;
  }
  return true;
}
"#;
    let (_tmp, target) = create_test_target(source, "test.js");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IT")
        .collect::<Vec<_>>();

    let replacements: Vec<String> = mutants
        .iter()
        .map(|m| m.new_text.trim().to_string())
        .collect();
    assert!(
        replacements
            .iter()
            .any(|text| text == "true" || text == "(true)"),
        "expected IT mutant to replace condition with true; found {replacements:?}"
    );
}

#[test]
fn it_mutation_preserves_parentheses_in_tsx() {
    let source = r#"
import type { FC } from "react";

const MaybeRender: FC<{ show: boolean; render(): JSX.Element }> = ({ show, render }) => {
  if (show && render) {
    return render();
  }
  return null;
};
"#;
    let (_tmp, target) = create_test_target(source, "test.tsx");
    let mutants = JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "IT")
        .collect::<Vec<_>>();

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "(show && render)" && m.new_text == "(true)"),
        "expected IT mutant to retain parentheses in TSX files"
    );
}
