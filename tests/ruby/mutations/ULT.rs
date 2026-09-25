use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn ult_rewrites_until_conditions_to_true() {
    let source = r#"
until value > 0
  value -= 1
end
"#;
    assert_only_slug_and_expected_new_texts(source, "ULT", &["true"]);
}

#[test]
fn ult_rewrites_until_modifier_conditions_to_true() {
    let source = r#"
value -= 1 until value > 0
"#;
    assert_only_slug_and_expected_new_texts(source, "ULT", &["true"]);
}
