//! Catches the case where a tree-sitter-daml bump leaves a snippet still
//! parseable but shifts the controller's AST position enough that mewt's walk
//! stops finding it, silently emitting zero mutants. Per-operator tests use
//! minimal snippets that miss this; the grammar repo's corpus proves parsing
//! but not engine traversal, so parser-only assertions belong there, not here.

use std::path::PathBuf;

use mewt::LanguageEngine;
use mewt::languages::daml::engine::DamlLanguageEngine;
use mewt::types::Target;

fn mutate_text(src: &str) -> Vec<mewt::types::Mutant> {
    let target = Target {
        id: 0,
        path: PathBuf::from("test.daml"),
        file_hash: mewt::types::Hash::digest(src.to_string()),
        text: src.to_string(),
        language: "daml"
            .parse()
            .expect("hardcoded language identifier should be valid"),
    };
    DamlLanguageEngine::new().mutate(&target)
}

#[test]
fn controller_next_to_record_with_expression_still_produces_mutants() {
    // A record-construction `with`-block in the do-body is a sibling of
    // the choice's controller. Confirm CPS still fires on the controller
    // regardless of the surrounding record-with idiom.
    let src = r#"module M where

template T
  with
    a : Party
    b : Party
    meta : Int
  where
    signatory a

    choice Build : Int
      controller a
      do
        let other = T with a, meta = 0
        return 1
"#;
    let mutants = mutate_text(src);
    let cps: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "CPS")
        .collect();
    assert!(
        !cps.is_empty(),
        "controller adjacent to a record-`with` expression should still produce CPS mutants; got {mutants:?}"
    );
}

#[test]
fn controller_below_key_inline_type_still_produces_cps() {
    // A template using `key s : Party` (a typed `key_decl`) sits next to a
    // controller. Confirm the controller below the key clause still parses
    // as a typed controller_decl and CPS fires on it.
    let src = r#"module M where

template T
  with
    owner : Party
    other : Party
    s : Text
  where
    signatory owner
    key s : Party
    maintainer owner

    choice Use : ()
      controller owner
      do return ()
"#;
    let mutants = mutate_text(src);
    let cps: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "CPS")
        .collect();
    assert!(
        !cps.is_empty(),
        "controller below `key s : Party` must still fire; got {mutants:?}. \
         If empty, either the inline-key parse regressed or the engine stopped \
         walking choices after a key clause."
    );
}
