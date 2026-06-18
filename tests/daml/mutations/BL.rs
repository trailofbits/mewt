use crate::daml::integration_tests::assert_only_slug_and_expected_new_texts;
use crate::daml::integration_tests::mutants_for_slug;

#[test]
fn bl_flips_true_to_false() {
    let source = r#"module M where

ready : Bool
ready = True
"#;
    assert_only_slug_and_expected_new_texts(source, "BL", &["False"]);
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
    // `True`/`False` share the `constructor` AST kind with user-defined
    // constructors; the engine matches on exact text so names like
    // `TrueColor` and `Nothing` must not flip.
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

#[test]
fn bl_flips_top_level_boolean_alongside_template() {
    let source = r#"module M where

flag : Bool
flag = True

template T
  with
    owner : Party
  where
    signatory owner
"#;
    assert_only_slug_and_expected_new_texts(source, "BL", &["False"]);
    let m = mutants_for_slug(source, "BL");
    assert_eq!(m.len(), 1, "expected one BL mutant, got {m:?}");
    assert_eq!(m[0].old_text, "True");
}

#[test]
fn bl_flips_boolean_in_choice_body() {
    let source = r#"module M where

template T
  with
    owner : Party
  where
    signatory owner

    choice Check : Bool
      controller owner
      do
        return True
"#;
    assert_only_slug_and_expected_new_texts(source, "BL", &["False"]);
    let m = mutants_for_slug(source, "BL");
    assert_eq!(m.len(), 1, "expected one BL mutant, got {m:?}");
    assert_eq!(m[0].old_text, "True");
}
