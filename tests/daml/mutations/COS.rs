use crate::daml::integration_tests::mutants_for_slug;
use std::collections::HashSet;

#[test]
fn cos_shuffles_comparison_operators() {
    let source = r#"module M where

f : Int -> Bool
f x = x == 0
"#;
    // DAML spells inequality `/=`, not `!=`.
    let cos = mutants_for_slug(source, "COS");
    assert_eq!(
        cos.len(),
        5,
        "COS should produce 5 replacements for ==: {cos:?}"
    );
    assert!(
        cos.iter().all(|m| m.old_text == "=="),
        "all COS mutants should replace ==: {cos:?}"
    );
    let new_ops: HashSet<&str> = cos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["/=", "<", "<=", ">", ">="]
            .into_iter()
            .collect::<HashSet<_>>(),
        "== should be replaced with every other comparison operator"
    );
}

#[test]
fn cos_shuffles_comparison_operators_inequality_side() {
    let source = r#"module M where

f : Int -> Bool
f x = x /= 0
"#;
    let cos = mutants_for_slug(source, "COS");
    assert_eq!(cos.len(), 5, "expected 5 COS mutants, got {cos:?}");
    assert!(
        cos.iter().all(|m| m.old_text == "/="),
        "all COS mutants should replace /=: {cos:?}"
    );
    let new_ops: HashSet<&str> = cos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["==", "<", "<=", ">", ">="]
            .into_iter()
            .collect::<HashSet<_>>(),
    );
}

#[test]
fn cos_shuffles_comparison_operators_relational_side() {
    let source = r#"module M where

f : Int -> Bool
f x = x < 0
"#;
    let cos = mutants_for_slug(source, "COS");
    assert_eq!(cos.len(), 5, "expected 5 COS mutants, got {cos:?}");
    assert!(cos.iter().all(|m| m.old_text == "<"));
    let new_ops: HashSet<&str> = cos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["==", "/=", "<=", ">", ">="]
            .into_iter()
            .collect::<HashSet<_>>(),
    );
}
