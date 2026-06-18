use std::collections::HashSet;

use crate::daml::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn new_texts(mutants: &[Mutant]) -> HashSet<&str> {
    mutants.iter().map(|m| m.new_text.as_str()).collect()
}

#[test]
fn sps_swaps_template_signatory_not_choice_controller() {
    // The template-level `signatory owner` is the SPS target; the choice's
    // `controller custodian` is not. SPS must rewrite `owner` (not `custodian`),
    // proving it keys off `signatory_decl`, not `controller_decl`.
    let source = r#"
module M where

template Vault
  with
    owner : Party
    custodian : Party
  where
    signatory owner

    choice Use : ()
      controller custodian
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert_eq!(m.len(), 1, "expected one SPS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "owner");
    assert_eq!(m[0].new_text, "custodian");
}

#[test]
fn sps_emits_one_mutant_per_alternative_party() {
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
    c : Party
  where
    signatory a

    choice Use : ()
      controller a
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert_eq!(m.len(), 2, "expected 2 SPS mutants, got {m:?}");
    assert_eq!(new_texts(&m), HashSet::from(["b", "c"]));
}

#[test]
fn sps_multi_party_swaps_every_position_and_excludes_already_in_list() {
    // Multi-party signatory with a single spare Party field: each listed
    // party swaps only to the party not already in the signatory list.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
    c : Party
  where
    signatory a, b

    choice Use : ()
      controller a
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert_eq!(m.len(), 2, "expected 2 SPS mutants, got {m:?}");
    let pairs: HashSet<(&str, &str)> = m
        .iter()
        .map(|m| (m.old_text.as_str(), m.new_text.as_str()))
        .collect();
    assert_eq!(pairs, HashSet::from([("a", "c"), ("b", "c")]));
}

#[test]
fn sps_multi_party_full_cross_product_with_unused_alternatives() {
    // template has {a, b, c, d}; signatory is `a, b`. Each position has two
    // alternatives (c, d) that aren't already in the list.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
    c : Party
    d : Party
  where
    signatory a, b

    choice Use : ()
      controller a
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert_eq!(m.len(), 4, "expected 4 SPS mutants, got {m:?}");
    let pairs: HashSet<(&str, &str)> = m
        .iter()
        .map(|m| (m.old_text.as_str(), m.new_text.as_str()))
        .collect();
    assert_eq!(
        pairs,
        HashSet::from([("a", "c"), ("a", "d"), ("b", "c"), ("b", "d")])
    );
}

#[test]
fn sps_emits_no_mutants_when_every_party_field_is_already_a_signatory() {
    // A multi-party signatory where the template exposes no Party field
    // outside the signatory list: every candidate is already a signatory.
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
      controller a
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert!(
        m.is_empty(),
        "no spare Party field outside the signatory list should produce no SPS mutants; got {m:?}"
    );
}

#[test]
fn sps_emits_no_mutants_when_template_has_only_one_party() {
    // Single-party signatory whose only in-scope Party field is the
    // signatory itself: no candidate to swap to.
    let source = r#"
module M where

template T
  with
    owner : Party
    balance : Decimal
  where
    signatory owner

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert!(
        m.is_empty(),
        "single-Party template should produce no SPS mutants; got {m:?}"
    );
}

#[test]
fn sps_aborts_on_projection_signatory() {
    // `signatory ref.owner` is a projection, not a bare variable. Even though
    // the template has spare Party fields, the whole site is dropped rather
    // than rewriting part of a non-trivial expression.
    let source = r#"
module M where

template T
  with
    owner : Party
    custodian : Party
  where
    signatory ref.owner

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert!(
        m.is_empty(),
        "projection signatory is not a plain party list; got {m:?}"
    );
}

#[test]
fn sps_per_template_scope_does_not_leak_across_templates() {
    // Two templates with disjoint Party params. A signatory swap inside
    // template A must only target template A's params, never template B's.
    let source = r#"
module M where

template A
  with
    alice : Party
    bob : Party
  where
    signatory alice

    choice Use : ()
      controller alice
      do
        return ()

template B
  with
    carol : Party
    dave : Party
  where
    signatory carol

    choice Use : ()
      controller carol
      do
        return ()
"#;
    let m = mutants_for_slug(source, "SPS");
    assert_eq!(m.len(), 2, "expected 2 SPS mutants, got {m:?}");
    let pairs: HashSet<(&str, &str)> = m
        .iter()
        .map(|m| (m.old_text.as_str(), m.new_text.as_str()))
        .collect();
    assert_eq!(pairs, HashSet::from([("alice", "bob"), ("carol", "dave")]));
}
