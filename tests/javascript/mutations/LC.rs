use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn lc_swaps_loop_control_keywords_in_js() {
    let source = r#"
function iterate(values) {
  for (const value of values) {
    if (!value) {
      break;
    }
    continue;
  }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "LC", &["break", "continue"]);
}

#[test]
fn lc_swaps_loop_control_keywords_in_ts() {
    let source = r#"
export function process(values: readonly number[]) {
  for (const value of values) {
    if (value < 0) {
      continue;
    }
    break;
  }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.ts", "LC", &["break", "continue"]);
}
