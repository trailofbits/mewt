use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn los_mutates_logical_operators_in_js() {
    let source = r#"
function choose(a, b, c) {
  return (a && b) || c;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "LOS", &["&&", "||"]);
}

#[test]
fn los_mutates_logical_operators_inside_tsx() {
    let source = r#"
import type { FC } from "react";

const Render: FC<{ ready: boolean; show: boolean }> = ({ ready, show }) => {
  return ready && show ? <div /> : null;
};
"#;

    assert_only_slug_and_expected_new_texts(source, "test.tsx", "LOS", &["||"]);
}
