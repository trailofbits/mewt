use std::collections::HashSet;

use crate::daml::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::daml::engine::DamlLanguageEngine;
use mewt::types::Mutant;

fn cpr_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    DamlLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "CPR")
        .collect()
}

/// Apply a mutant to the source it came from and return the resulting
/// program text. Useful for asserting the post-state of a removal.
fn apply(source: &str, mutant: &Mutant) -> String {
    let start = mutant.byte_offset as usize;
    let end = start + mutant.old_text.len();
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..start]);
    out.push_str(&mutant.new_text);
    out.push_str(&source[end..]);
    out
}

#[test]
fn cpr_drops_one_party_from_a_two_party_controller() {
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a, b

    choice Use : ()
      controller a, b
      do
        return ()
"#;
    let m = cpr_mutants(source);
    assert_eq!(m.len(), 2, "expected 2 CPR mutants, got {m:?}");

    let results: HashSet<String> = m
        .iter()
        .map(|mu| {
            let mutated = apply(source, mu);
            // Pull out the rewritten `controller ...` line for a stable check.
            mutated
                .lines()
                .find(|l| l.trim_start().starts_with("controller"))
                .unwrap()
                .trim()
                .to_string()
        })
        .collect();

    assert_eq!(
        results,
        HashSet::from(["controller a".to_string(), "controller b".to_string()])
    );
}

#[test]
fn cpr_drops_each_party_from_a_three_party_controller() {
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
    c : Party
  where
    signatory a, b, c

    choice Use : ()
      controller a, b, c
      do
        return ()
"#;
    let m = cpr_mutants(source);
    assert_eq!(m.len(), 3, "expected 3 CPR mutants, got {m:?}");

    let results: HashSet<String> = m
        .iter()
        .map(|mu| {
            apply(source, mu)
                .lines()
                .find(|l| l.trim_start().starts_with("controller"))
                .unwrap()
                .trim()
                .to_string()
        })
        .collect();

    // Each removal preserves the other two parties in their original order.
    assert_eq!(
        results,
        HashSet::from([
            "controller b, c".to_string(),
            "controller a, c".to_string(),
            "controller a, b".to_string(),
        ])
    );
}

#[test]
fn cpr_emits_no_mutants_for_single_party_controllers() {
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a

    choice Use : ()
      controller a
      do
        return ()
"#;
    let m = cpr_mutants(source);
    assert!(
        m.is_empty(),
        "single-party controllers must not produce CPR mutants \
        (dropping the only party leaves the choice with no controller \
        and doesn't compile); got {m:?}"
    );
}

#[test]
fn cpr_only_fires_inside_multi_party_lists() {
    // The first choice has a single-party controller; the second has two.
    // Only the second should produce CPR mutants.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a, b

    choice Solo : ()
      controller a
      do
        return ()

    choice Joint : ()
      controller a, b
      do
        return ()
"#;
    let m = cpr_mutants(source);
    assert_eq!(
        m.len(),
        2,
        "expected 2 CPR mutants (from the Joint site only), got {m:?}"
    );
}

#[test]
fn cpr_separator_whitespace_is_consumed_by_the_removal() {
    // Whatever whitespace surrounds the comma must be swallowed by the
    // removal so the resulting controller line is well-formed (no
    // dangling commas, no double spaces).
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a, b

    choice Tight : ()
      controller a,b
      do
        return ()

    choice Wide : ()
      controller a   ,   b
      do
        return ()
"#;
    let m = cpr_mutants(source);
    assert_eq!(m.len(), 4, "expected 4 CPR mutants (2 per site), got {m:?}");

    let results: HashSet<String> = m
        .iter()
        .map(|mu| {
            apply(source, mu)
                .lines()
                .find(|l| l.contains("controller") && !l.contains(','))
                .unwrap()
                .trim()
                .to_string()
        })
        .collect();

    assert_eq!(
        results,
        HashSet::from(["controller a".to_string(), "controller b".to_string()])
    );
}

#[test]
fn cpr_does_not_offer_removal_that_leaves_party_set_unchanged() {
    // `controller a, a` names the same authoriser twice, so dropping either
    // `a` leaves `controller a`: the authorization set is unchanged
    // (equivalent mutant). CPR must offer no removal here.
    let source = r#"
module M where

template T
  with
    a : Party
  where
    signatory a

    choice Use : ()
      controller a, a
      do
        return ()
"#;
    let m = cpr_mutants(source);
    for mu in &m {
        let line = apply(source, mu)
            .lines()
            .find(|l| l.trim_start().starts_with("controller"))
            .unwrap()
            .trim()
            .to_string();
        assert_ne!(
            line, "controller a, a",
            "removal must change the controller line; got {mu:?}"
        );
        assert_ne!(
            line, "controller a",
            "removing a duplicate `a` leaves the authorization set unchanged \
            (equivalent mutant); CPR must not offer it. got {mu:?}"
        );
    }
    assert!(
        m.is_empty(),
        "`controller a, a` should produce no CPR mutants; got {m:?}"
    );
}

#[test]
fn cpr_does_not_fire_on_parenthesised_controller() {
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a

    choice Use : ()
      controller (a)
      do
        return ()
"#;
    let m = cpr_mutants(source);
    assert!(
        m.is_empty(),
        "parenthesised controller is not a plain party list; got {m:?}"
    );
}
