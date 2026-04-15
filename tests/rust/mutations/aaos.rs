use crate::rust::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn aaos_mutates_arithmetic_assignments() {
    let source = r#"
fn demo(mut a: i32, b: i32) {
    a += b;
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AAOS", &["-=", "*=", "/=", "%="]);
}
