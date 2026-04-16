use crate::rust::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn saos_mutates_shift_assignments() {
    let source = r#"
fn demo(mut a: u8) {
    a <<= 1;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "SAOS", &[">>="]);
}
