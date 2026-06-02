use crate::daml::integration_tests::mutants_for_slug;

#[test]
fn los_shuffles_logical_operators() {
    let source = r#"module M where

f : Bool -> Bool -> Bool
f a b = a && b
"#;
    let m = mutants_for_slug(source, "LOS");
    assert_eq!(m.len(), 1, "expected one LOS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "&&");
    assert_eq!(m[0].new_text, "||");
}

#[test]
fn los_shuffles_logical_operators_reverse() {
    // Reverse direction: guards `||` dropping out of the operator set.
    let source = r#"module M where

g : Bool -> Bool -> Bool
g a b = a || b
"#;
    let m = mutants_for_slug(source, "LOS");
    assert_eq!(m.len(), 1, "expected one LOS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "||");
    assert_eq!(m[0].new_text, "&&");
}
