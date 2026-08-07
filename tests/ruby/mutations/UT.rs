use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn ut_rewrites_unless_conditions_to_true() {
    let source = r#"
unless value > 0
  puts value
end
"#;
    assert_only_slug_and_expected_new_texts(source, "UT", &["true"]);
}

#[test]
fn ut_rewrites_unless_modifier_conditions_to_true() {
    let source = r#"
puts value unless value > 0
"#;
    assert_only_slug_and_expected_new_texts(source, "UT", &["true"]);
}
