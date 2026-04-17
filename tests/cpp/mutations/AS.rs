use crate::cpp::integration_tests::mutants_for_slug;

#[test]
fn test_as_replacement_content() {
    let source = r#"
int add(int a, int b) { return a + b; }
int main() {
    return add(10, 20);
}
"#;
    let as_mut = mutants_for_slug(source, "AS");
    assert_eq!(
        as_mut.len(),
        1,
        "AS should produce 1 swap for add(10, 20): {as_mut:?}"
    );
    assert!(
        as_mut[0].old_text.contains("10") && as_mut[0].old_text.contains("20"),
        "AS old_text should contain both args: {:?}",
        as_mut[0].old_text
    );
    assert!(
        as_mut[0].new_text.starts_with("20") && as_mut[0].new_text.ends_with("10"),
        "AS should swap argument order: {:?}",
        as_mut[0].new_text
    );
}
