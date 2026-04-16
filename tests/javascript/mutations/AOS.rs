use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn aos_mutates_arithmetic_operators_in_js() {
    let source = r#"
function combine(a, b) {
  return a + b;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "AOS", &["-", "*", "/", "%", "**"]);
}

#[test]
fn aos_mutates_arithmetic_operators_in_ts() {
    let source = r#"
export function calculate(value: number, delta: number): number {
  return value + delta;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.ts", "AOS", &["-", "*", "/", "%", "**"]);
}
