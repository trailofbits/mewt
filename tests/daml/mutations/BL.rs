use crate::daml::integration_tests::assert_only_slug_and_expected_new_texts;
use crate::daml::integration_tests::mutants_for_slug;

#[test]
fn bl_flips_true_to_false() {
    let source = r#"module M where

ready : Bool
ready = True
"#;
    assert_only_slug_and_expected_new_texts(source, "BL", &["False"]);
    // Tighten: the flipped literal must be the bare `True`, not some larger node.
    let m = mutants_for_slug(source, "BL");
    assert_eq!(m.len(), 1, "expected one BL mutant, got {m:?}");
    assert_eq!(m[0].old_text, "True");
}

#[test]
fn bl_flips_false_to_true() {
    let source = r#"module M where

ready : Bool
ready = False
"#;
    assert_only_slug_and_expected_new_texts(source, "BL", &["True"]);
    let m = mutants_for_slug(source, "BL");
    assert_eq!(m.len(), 1, "expected one BL mutant, got {m:?}");
    assert_eq!(m[0].old_text, "False");
}

#[test]
fn bl_flips_only_real_booleans_not_other_constructors() {
    // `True`/`False` are ordinary data constructors in DAML, sharing the
    // `constructor` parse-tree kind with `Just`, `Nothing`, and user-defined
    // constructors. The engine matches on the exact text `True`/`False`, so
    // `TrueColor`/`FalseAlarm` (names that merely *contain* True/False) and
    // `Just`/`Nothing` must be left untouched - only the real `True` flips.
    let source = r#"module M where

data Color = TrueColor | FalseAlarm

pick : Optional Int -> Color
pick Nothing = TrueColor
pick (Just _) = FalseAlarm

ready : Bool
ready = True
"#;
    let m = mutants_for_slug(source, "BL");
    assert_eq!(
        m.len(),
        1,
        "exactly one real boolean should flip; got {m:?}"
    );
    assert_eq!(m[0].old_text, "True");
    assert_eq!(m[0].new_text, "False");
}

#[test]
fn bl_emits_nothing_when_there_are_no_boolean_literals() {
    // No `True`/`False` anywhere - only constructors whose names contain or
    // resemble the booleans. The engine must produce zero BL mutants.
    let source = r#"module M where

data Color = TrueColor | FalseAlarm

pick : Optional Int -> Color
pick Nothing = TrueColor
pick (Just _) = FalseAlarm
"#;
    let m = mutants_for_slug(source, "BL");
    assert!(
        m.is_empty(),
        "no boolean literals means no BL mutants; got {m:?}"
    );
}
