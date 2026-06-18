use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn ncr_swaps_nullish_and_logical_defaulting_in_js() {
    let source = r#"
function defaults(a, b, c) {
  return (a ?? b) || c;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "NCR", &["??", "||"]);
}

#[test]
fn ncr_is_available_in_typescript_dialects() {
    let source = r#"
type Box = { value?: number };
const render = (box: Box, fallback: number) => box.value ?? fallback;
"#;

    assert_only_slug_and_expected_new_texts(source, "test.ts", "NCR", &["||"]);
}
