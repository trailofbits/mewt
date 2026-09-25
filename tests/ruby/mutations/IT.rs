use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn it_rewrites_conditions_to_true() {
    let source = r#"
if value > 0
  puts value
end
"#;
    assert_only_slug_and_expected_new_texts(source, "IT", &["true"]);
}

#[test]
fn it_rewrites_modifier_conditions_to_true() {
    let source = r#"
puts value if value > 0
"#;
    assert_only_slug_and_expected_new_texts(source, "IT", &["true"]);
}
