use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn saos_mutates_left_shift_assignments() {
    let source = r#"
function f(a) {
  a <<= 1;
  return a;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "SAOS", &[">>=", ">>>="]);
}

#[test]
fn saos_mutates_unsigned_shift_assignments_in_ts() {
    let source = r#"
export function wrap(mask: number) {
  mask >>>= 1;
  return mask;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.ts", "SAOS", &["<<=", ">>="]);
}
