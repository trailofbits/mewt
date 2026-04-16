use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn sos_mutates_shift_operators_in_js() {
    let source = r#"
function shift(value) {
  return value << 2;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "SOS", &[">>", ">>>"]);
}

#[test]
fn sos_mutates_shift_operators_in_ts() {
    let source = r#"
export function rotate(value: number, amount: number): number {
  return (value >> amount) & 0xff;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.ts", "SOS", &["<<", ">>>"]);
}
