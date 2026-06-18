use crate::daml::integration_tests::mutants_for_slug;
use std::collections::HashSet;

#[test]
fn aos_shuffles_arithmetic_operators() {
    let source = r#"module M where

f : Int -> Int
f x = x + 1
"#;
    let m = mutants_for_slug(source, "AOS");
    assert_eq!(m.len(), 3, "expected 3 AOS mutants, got {m:?}");
    assert!(
        m.iter().all(|mu| mu.old_text == "+"),
        "all AOS mutants should replace +: {m:?}"
    );
    let new_ops: HashSet<&str> = m.iter().map(|mu| mu.new_text.as_str()).collect();
    assert_eq!(new_ops, ["-", "*", "/"].into_iter().collect::<HashSet<_>>());
}

#[test]
fn aos_shuffles_arithmetic_operators_reverse() {
    let source = r#"module M where

f : Int -> Int
f x = x - 1
"#;
    let m = mutants_for_slug(source, "AOS");
    assert_eq!(m.len(), 3, "expected 3 AOS mutants, got {m:?}");
    assert!(
        m.iter().all(|mu| mu.old_text == "-"),
        "all AOS mutants should replace -: {m:?}"
    );
    let new_ops: HashSet<&str> = m.iter().map(|mu| mu.new_text.as_str()).collect();
    assert_eq!(new_ops, ["+", "*", "/"].into_iter().collect::<HashSet<_>>());
}

#[test]
fn aos_shuffles_multiplication() {
    let source = r#"module M where

f : Int -> Int
f x = x * 2
"#;
    let m = mutants_for_slug(source, "AOS");
    assert_eq!(m.len(), 3, "expected 3 AOS mutants, got {m:?}");
    assert!(
        m.iter().all(|mu| mu.old_text == "*"),
        "all AOS mutants should replace *: {m:?}"
    );
    let new_ops: HashSet<&str> = m.iter().map(|mu| mu.new_text.as_str()).collect();
    assert_eq!(new_ops, ["+", "-", "/"].into_iter().collect::<HashSet<_>>());
}

#[test]
fn aos_shuffles_division() {
    let source = r#"module M where

f : Int -> Int
f x = x / 2
"#;
    let m = mutants_for_slug(source, "AOS");
    assert_eq!(m.len(), 3, "expected 3 AOS mutants, got {m:?}");
    assert!(
        m.iter().all(|mu| mu.old_text == "/"),
        "all AOS mutants should replace /: {m:?}"
    );
    let new_ops: HashSet<&str> = m.iter().map(|mu| mu.new_text.as_str()).collect();
    assert_eq!(new_ops, ["+", "-", "*"].into_iter().collect::<HashSet<_>>());
}
