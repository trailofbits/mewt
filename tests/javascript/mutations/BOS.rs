use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bos_mutates_bitwise_operators_in_js() {
    let source = r#"
function combine(flags, mask) {
  return flags & mask;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "BOS", &["|", "^"]);
}

#[test]
fn bos_mutates_bitwise_operators_in_ts() {
    let source = r#"
export function overlaps(flags: number, permission: number): boolean {
  return (flags & permission) !== 0;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.ts", "BOS", &["|", "^"]);
}
