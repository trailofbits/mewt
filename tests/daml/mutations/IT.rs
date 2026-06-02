use crate::daml::integration_tests::mutants_for_slug;

#[test]
fn it_hardcodes_condition_to_true() {
    let source = r#"module M where

f : Int -> Int
f x = if x > 0 then x else 0
"#;
    let m = mutants_for_slug(source, "IT");
    assert_eq!(m.len(), 1, "expected one IT mutant, got {m:?}");
    assert_eq!(m[0].old_text, "x > 0");
    assert_eq!(m[0].new_text, "True");
}
