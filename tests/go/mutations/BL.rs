use crate::go::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bl_flips_boolean_literals() {
    let source = r#"
package main

func values() (bool, bool) {
    return true, false
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BL", &["false", "true"]);
}
