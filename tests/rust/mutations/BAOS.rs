use crate::rust::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn baos_mutates_bitwise_assignments() {
    let source = r#"
fn demo(mut a: u8, b: u8) {
    a &= b;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BAOS", &["|=", "^="]);
}
