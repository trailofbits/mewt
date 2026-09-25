use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn if_rewrites_conditions_to_false() {
    let source = r#"
if value > 0
  puts value
end
"#;
    assert_only_slug_and_expected_new_texts(source, "IF", &["false"]);
}

#[test]
fn if_rewrites_modifier_conditions_to_false() {
    let source = r#"
puts value if value > 0
"#;
    assert_only_slug_and_expected_new_texts(source, "IF", &["false"]);
}

#[test]
fn if_rewrites_elsif_conditions_to_false() {
    let source = r#"
if x > 10
  puts 1
elsif x > 5
  puts 2
end
"#;
    assert_only_slug_and_expected_new_texts(source, "IF", &["false"]);
}

#[test]
fn if_rewrites_if_guard_conditions_to_false() {
    let source = r#"
case value
in [a, b] if a > b
  puts a
end
"#;
    assert_only_slug_and_expected_new_texts(source, "IF", &["false"]);
}
