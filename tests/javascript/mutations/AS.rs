use crate::javascript::integration_tests::{
    assert_only_slug_and_expected_new_texts, create_test_target,
};
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn as_swaps_adjacent_arguments_in_function_calls() {
    let source = r#"
function callAll() {
  consume(foo(prepare(), value, other));
}
"#;

    assert_only_slug_and_expected_new_texts(
        source,
        "test.js",
        "AS",
        &["value, prepare()", "other, value"],
    );
}

#[test]
fn as_swaps_arguments_inside_tsx_call_expressions() {
    let source = r#"
import { h } from "preact";

const element = h(Component, {
  props: buildProps(fetchData(), transform(data)),
});
"#;
    let (_tmp, target) = create_test_target(source, "test.tsx");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let as_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "AS").collect();
    assert!(
        !as_mutants.is_empty(),
        "expected AS mutants to be generated inside TSX call expressions"
    );
    assert!(
        as_mutants
            .iter()
            .any(|m| m.new_text.contains("transform(data), fetchData()")),
        "expected AS mutant swapping adjacent TSX arguments: {as_mutants:?}"
    );
}
