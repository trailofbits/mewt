use std::collections::HashSet;

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

#[test]
fn it_fires_once_per_live_conditional_and_skips_commented_ones() {
    let source = r#"module M where

f : Int -> Int
f x = if x > 0 then x else 0

g : Int -> Int -> Int
g x y = if x > y then x else y

-- if commented then 1 else 2
h : Int
h = 7
"#;
    let m = mutants_for_slug(source, "IT");
    assert_eq!(
        m.len(),
        2,
        "expected one IT mutant per live conditional, got {m:?}"
    );
    let conditions: HashSet<&str> = m.iter().map(|mu| mu.old_text.as_str()).collect();
    assert_eq!(conditions, HashSet::from(["x > 0", "x > y"]));
    assert!(
        m.iter().all(|mu| mu.new_text == "True"),
        "every IT mutant rewrites the condition to True; got {m:?}"
    );
}
