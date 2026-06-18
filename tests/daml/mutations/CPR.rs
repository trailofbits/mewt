use std::collections::HashSet;

use crate::daml::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

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

/// Apply `mutant` to `source` and return the first line that starts with
/// `controller`, trimmed. Single-controller fixtures use this to assert the
/// rewritten controller clause; multi-site fixtures need a different filter.
fn mutated_controller_line(source: &str, mutant: &Mutant) -> String {
    apply(source, mutant)
        .lines()
        .find(|l| l.trim_start().starts_with("controller"))
        .unwrap()
        .trim()
        .to_string()
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
    let m = mutants_for_slug(source, "CPR");
    assert_eq!(m.len(), 2, "expected 2 CPR mutants, got {m:?}");

    let results: HashSet<String> = m
        .iter()
        .map(|mu| mutated_controller_line(source, mu))
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
    let m = mutants_for_slug(source, "CPR");
    assert_eq!(m.len(), 3, "expected 3 CPR mutants, got {m:?}");

    let results: HashSet<String> = m
        .iter()
        .map(|mu| mutated_controller_line(source, mu))
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
    let m = mutants_for_slug(source, "CPR");
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
    let m = mutants_for_slug(source, "CPR");
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
    let m = mutants_for_slug(source, "CPR");
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
    let m = mutants_for_slug(source, "CPR");
    for mu in &m {
        let line = mutated_controller_line(source, mu);
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
fn cpr_emits_equivalent_mutants_on_parenthesised_duplicate() {
    // `controller (a), a`: `(a)` and `a` are semantically the same party
    // (parens are transparent), but CPR's dedup at engine.rs compares surface
    // text only, so `"(a)" != "a"` and neither removal is skipped. We emit
    // two equivalent mutants here and accept them: this matches the CPS side
    // (`cps_swaps_whole_parenthesised_controller` also asserts `(a) -> a` as
    // an equivalent mutant). A future shape-aware dedup could suppress them.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a

    choice Use : ()
      controller (a), a
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPR");
    assert_eq!(m.len(), 2, "expected 2 CPR mutants, got {m:?}");
    let results: HashSet<String> = m
        .iter()
        .map(|mu| mutated_controller_line(source, mu))
        .collect();
    assert_eq!(
        results,
        HashSet::from(["controller a".to_string(), "controller (a)".to_string()])
    );
}

#[test]
fn cpr_drops_parenthesised_party_from_multi_party_list() {
    // `controller (a), b`: the parens-wrapped party is removable like a
    // bare variable. Removal byte range for idx=0 includes the open paren
    // through the trailing separator; idx=1 takes the leading separator.
    // The single-party `controller (a)` case is covered by the generic
    // `cpr_emits_no_mutants_for_single_party_controllers` test.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a

    choice Use : ()
      controller (a), b
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPR");
    assert_eq!(m.len(), 2, "expected 2 CPR mutants, got {m:?}");
    let results: HashSet<String> = m
        .iter()
        .map(|mu| mutated_controller_line(source, mu))
        .collect();
    assert_eq!(
        results,
        HashSet::from(["controller b".to_string(), "controller (a)".to_string(),])
    );
}

#[test]
fn cpr_drops_projection_party_from_multi_party_list() {
    // `controller ref.actor, b`: projection-shaped parties are removable like
    // bare variables. Pins the byte-range arithmetic at `removal_byte_range`
    // and the text-equality dedup against non-atomic party expressions: a
    // regression that, say, used `node_text` to find separators inside a
    // projection's bytes would break this test's exact-output assertion.
    let source = r#"
module M where

template T
  with
    b : Party
  where
    signatory b

    choice Use : ()
      controller ref.actor, b
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPR");
    assert_eq!(m.len(), 2, "expected 2 CPR mutants, got {m:?}");
    let results: HashSet<String> = m
        .iter()
        .map(|mu| mutated_controller_line(source, mu))
        .collect();
    assert_eq!(
        results,
        HashSet::from([
            "controller b".to_string(),
            "controller ref.actor".to_string(),
        ])
    );
}
