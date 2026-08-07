use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn uf_rewrites_unless_conditions_to_false() {
    let source = r#"
unless value > 0
  puts value
end
"#;
    assert_only_slug_and_expected_new_texts(source, "UF", &["false"]);
}

#[test]
fn uf_rewrites_unless_modifier_conditions_to_false() {
    let source = r#"
puts value unless value > 0
"#;
    assert_only_slug_and_expected_new_texts(source, "UF", &["false"]);
}

#[test]
fn uf_rewrites_unless_guard_conditions_to_false() {
    let source = r#"
case value
in [a, b] unless a > b
  puts a
end
"#;
    assert_only_slug_and_expected_new_texts(source, "UF", &["false"]);
}
