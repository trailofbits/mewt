use crate::javascript::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn saos_mutates_shift_assignments() {
    let source = r#"
function f(a) {
  a <<= 1;
  return a;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "test.js", "SAOS", &[">>="]);
}
