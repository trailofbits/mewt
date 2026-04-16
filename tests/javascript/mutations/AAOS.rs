use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn aaos_mutates_arithmetic_assignments() {
    let source = r#"
function f(a, b) {
  a += b;
  return a;
}
"#;

    assert_only_slug_and_expected_new_texts(
        source,
        "test.js",
        "AAOS",
        &["-=", "*=", "/=", "%=", "**="],
    );
}

#[test]
fn aaos_mutates_arithmetic_assignments_in_ts() {
    let source = r#"
export function update(counter: number, delta: number): number {
  counter += delta;
  return counter;
}
"#;

    assert_only_slug_and_expected_new_texts(
        source,
        "test.ts",
        "AAOS",
        &["-=", "*=", "/=", "%=", "**="],
    );
}
