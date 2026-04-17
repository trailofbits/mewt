use crate::cpp::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "CR")
}

#[test]
fn test_comment_replacement() {
    let source = r#"
int f() {
    int x = 42;
    return x;
}
"#;
    let cr = slug_mutants(source);
    assert!(!cr.is_empty(), "Should generate CR mutations");
    for m in &cr {
        assert!(
            m.new_text.starts_with("/*") && m.new_text.ends_with("*/"),
            "CR should wrap in block comments: {}",
            m.new_text
        );
    }
}

#[test]
fn cr_does_not_produce_nested_comments() {
    let source = r#"
// int x = 1;
/* int y = 2; */
int main() {
    return 0;
}
"#;
    let cr = slug_mutants(source);
    for m in &cr {
        assert!(
            !m.new_text.contains("/* /*") && !m.new_text.contains("*/ */"),
            "CR should not produce nested block comments: {:?}",
            m.new_text
        );
    }
}
