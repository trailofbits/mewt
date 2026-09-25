use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn wf_rewrites_conditions_to_false() {
    let source = r#"
while value > 0
  value -= 1
end
"#;
    assert_only_slug_and_expected_new_texts(source, "WF", &["false"]);
}

#[test]
fn wf_rewrites_modifier_conditions_to_false() {
    let source = r#"
value -= 1 while value > 0
"#;
    assert_only_slug_and_expected_new_texts(source, "WF", &["false"]);
}
