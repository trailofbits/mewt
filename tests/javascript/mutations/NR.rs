use crate::javascript::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn nr_removes_negation_operator() {
    let source = r#"
function check(flag) {
  if (!flag) {
    throw new Error("bad");
  }
  return !(flag && true);
}
"#;
    let (_dir, target) = create_test_target(source, "test.js");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert_eq!(nr.len(), 2, "Should generate exactly 2 NR mutations");
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!flag" && m.new_text == "flag"),
        "NR should replace !flag with flag: {nr:?}"
    );
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(flag && true)" && m.new_text == "(flag && true)"),
        "NR should replace !(flag && true) with (flag && true): {nr:?}"
    );
}

#[test]
fn nr_ignores_other_unary_operators() {
    let source = r#"
function negate(x) {
  return -x;
}
"#;
    let (_dir, target) = create_test_target(source, "test.js");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert!(nr.is_empty(), "NR should not trigger on - unary operator");
}

#[test]
fn nr_ignores_negation_inside_comments() {
    let source = r#"
// if (!flag) { throw new Error(); }
/* !x */
function noop() {}
"#;
    let (_dir, target) = create_test_target(source, "test.ts");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert!(
        nr.is_empty(),
        "NR should not generate mutations inside comments"
    );
}
