use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bl_flips_true_to_false() {
    let source = r#"
const featureEnabled = true;
if (featureEnabled) {
  perform();
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "BL", &["false"]);
}

#[test]
fn bl_flips_false_to_true_in_tsx() {
    let source = r#"
import type { FC } from "react";

const Toggle: FC<{ enabled?: boolean }> = ({ enabled = false }) => {
  return enabled ? <div>On</div> : <div>Off</div>;
};
"#;

    assert_only_slug_and_expected_new_texts(source, "test.tsx", "BL", &["true"]);
}
