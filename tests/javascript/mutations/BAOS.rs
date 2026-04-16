use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn baos_mutates_bitwise_assignments() {
    let source = r#"
function f(a, b) {
  a &= b;
  return a;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "BAOS", &["|=", "^="]);
}

#[test]
fn baos_mutates_bitwise_assignments_in_ts() {
    let source = r#"
export function configure(mask: number, flag: number): number {
  mask &= flag;
  return mask;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.ts", "BAOS", &["|=", "^="]);
}
