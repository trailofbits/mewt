use std::collections::HashSet;

use crate::daml::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn new_texts(mutants: &[Mutant]) -> HashSet<&str> {
    mutants.iter().map(|m| m.new_text.as_str()).collect()
}

#[test]
fn cps_swaps_single_party_controller_to_other_template_party() {
    let source = r#"
module M where

template Vault
  with
    owner : Party
    custodian : Party
  where
    signatory owner

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 1, "expected one CPS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "owner");
    assert_eq!(m[0].new_text, "custodian");
}

#[test]
fn cps_emits_one_mutant_per_alternative_party() {
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
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 2, "expected 2 CPS mutants, got {m:?}");
    assert_eq!(new_texts(&m), HashSet::from(["b", "c"]));
}

#[test]
fn cps_multi_party_swaps_every_position_and_excludes_already_in_list() {
    // template has {a, b, c}; controller is `a, b`. CPS should emit one
    // mutant per position, swapping to the only Party not already in the
    // list (c). No `c, c`, no `a, a` - duplicates are excluded at source.
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
      controller a, b
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 2, "expected 2 CPS mutants, got {m:?}");
    let pairs: HashSet<(&str, &str)> = m
        .iter()
        .map(|m| (m.old_text.as_str(), m.new_text.as_str()))
        .collect();
    assert_eq!(pairs, HashSet::from([("a", "c"), ("b", "c")]));
}

#[test]
fn cps_multi_party_full_cross_product_with_unused_alternatives() {
    // template has {a, b, c, d}; controller is `a, b`. Each position has
    // two alternatives (c, d) that aren't already in the list.
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
      controller a, b
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 4, "expected 4 CPS mutants, got {m:?}");
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
fn cps_emits_no_mutants_when_template_has_only_one_party() {
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
    let m = mutants_for_slug(source, "CPS");
    assert!(
        m.is_empty(),
        "single-Party template should produce no CPS mutants; got {m:?}"
    );
}

#[test]
fn cps_handles_whitespace_variations_in_separator() {
    // Whitespace around the comma between parties must not affect the mutants.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
    c : Party
  where
    signatory a

    choice Tight : ()
      controller a,b
      do
        return ()

    choice Wide : ()
      controller a ,   b
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    // Each of the two controllers gets two position-aware swaps to `c`.
    assert_eq!(
        m.len(),
        4,
        "whitespace variations should not change the mutant count; got {m:?}"
    );
    let news = new_texts(&m);
    assert_eq!(news, HashSet::from(["c"]));
}

#[test]
fn cps_multi_line_layout_controller_produces_both_parties() {
    // Party list broken across lines with a leading comma on the continuation.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
    c : Party
  where
    signatory a

    choice Wrapped : ()
      controller
        a
      , b
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(
        m.len(),
        2,
        "multi-line layout site should produce 2 mutants once the grammar accepts the continuation; got {m:?}"
    );
    let news = new_texts(&m);
    assert_eq!(news, HashSet::from(["c"]));
    let olds: HashSet<&str> = m.iter().map(|mu| mu.old_text.as_str()).collect();
    assert_eq!(olds, HashSet::from(["a", "b"]));
}

#[test]
fn cps_per_template_scope_does_not_leak_across_templates() {
    // Two templates with disjoint Party params. A swap inside template A
    // must only target template A's params, never template B's.
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
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 2, "expected 2 CPS mutants, got {m:?}");
    let pairs: HashSet<(&str, &str)> = m
        .iter()
        .map(|m| (m.old_text.as_str(), m.new_text.as_str()))
        .collect();
    assert_eq!(pairs, HashSet::from([("alice", "bob"), ("carol", "dave")]));
}

#[test]
fn cps_targets_choice_local_party_parameters_within_their_own_choice() {
    // `actor` is declared on Reassign only; it must not leak into Cancel's scope.
    let source = r#"
module M where

template Escrow
  with
    primary : Party
  where
    signatory primary

    choice Reassign : ()
      with
        actor : Party
      controller primary
      do
        return ()

    choice Cancel : ()
      controller primary
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");

    assert!(
        m.iter()
            .any(|mu| mu.old_text == "primary" && mu.new_text == "actor"),
        "choice-local `actor` must be a swap target for the Reassign \
        controller; got {m:?}"
    );

    // Exactly one `actor` target (at Reassign); a count of 2 would mean it
    // leaked into Cancel's scope.
    let actor_targets: Vec<&Mutant> = m.iter().filter(|mu| mu.new_text == "actor").collect();
    assert_eq!(
        actor_targets.len(),
        1,
        "choice-local `actor` must be offered only at its own (Reassign) \
        controller, not at Cancel; got {actor_targets:?}"
    );
    assert_eq!(actor_targets[0].old_text, "primary");

    assert_eq!(
        m.len(),
        1,
        "expected exactly one CPS mutant (Reassign `primary` -> `actor`); \
        got {m:?}"
    );
}

#[test]
fn cps_collects_party_param_after_a_field_named_choice() {
    // `choice` is a soft keyword; a field literally named `choice : Int` must
    // not end the with-block, so `custodian` is still collected after it.
    let source = r#"
module M where

template T
  with
    owner : Party
    choice : Int
    custodian : Party
  where
    signatory owner

    choice Transfer : ContractId T
      controller owner
      do create this
"#;
    let m = mutants_for_slug(source, "CPS");
    assert!(
        m.iter()
            .any(|mu| mu.old_text == "owner" && mu.new_text == "custodian"),
        "custodian (declared after the `choice` field) must be a swap target \
        for owner; got {m:?}"
    );
}

#[test]
fn cps_emits_nothing_for_function_application_controller() {
    // A function-application controller is not a plain party list, so we must
    // not mistake the function name `resolveActor` for a party. The template
    // has two Party params, so WITHOUT the fix a swap would be emitted; with
    // it, neither CPS nor CPR fires.
    let source = r#"
module M where

resolveActor : Party -> Party
resolveActor p = p

template Asset
  with
    owner : Party
    custodian : Party
  where
    signatory owner

    choice Transfer : ()
      controller resolveActor owner
      do return ()
"#;
    assert!(
        mutants_for_slug(source, "CPS").is_empty(),
        "function-application controller should produce no CPS mutants"
    );
    assert!(
        mutants_for_slug(source, "CPR").is_empty(),
        "function-application controller should produce no CPR mutants"
    );
}

#[test]
fn cps_ignores_function_typed_binding_whose_type_starts_with_party() {
    // `notify : Party -> ()` is function-typed despite starting with `Party`;
    // it must not be a swap target.
    let source = r#"
module M where

template T
  with
    owner : Party
    notify : Party -> ()
    custodian : Party
  where
    signatory owner

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    let news = new_texts(&m);
    assert!(
        !news.contains("notify"),
        "function-typed `notify : Party -> ()` must not be a CPS swap target; \
        got {m:?}"
    );
    // The only legitimate swap target is the real Party `custodian`.
    assert_eq!(
        news,
        HashSet::from(["custodian"]),
        "expected the only swap target to be the real Party `custodian`; got {m:?}"
    );
}

#[test]
fn cps_aborts_on_parenthesised_controller() {
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
    let m = mutants_for_slug(source, "CPS");
    assert!(
        m.is_empty(),
        "parenthesised controller should produce no CPS mutants; got {m:?}"
    );
}

#[test]
fn cps_mutates_through_a_trailing_line_comment() {
    // The trailing `--` comment is a sibling node, not part of the controller.
    let source = r#"
module M where

template T
  with
    a : Party
    b : Party
  where
    signatory a

    choice Use : ()
      controller a -- the original
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(
        m.len(),
        1,
        "controller followed by an inline comment is unambiguous in the typed AST; \
         CPS should produce exactly the `a -> b` mutant. got {m:?}"
    );
    assert_eq!(m[0].old_text, "a");
    assert_eq!(m[0].new_text, "b");
}

#[test]
fn cps_old_text_is_the_bare_party_identifier_not_the_full_line() {
    // The replacement must target the party variable's exact byte range -
    // not the whole `controller ...` line - so the mutant's diff is minimal
    // and the surrounding whitespace / comments / siblings stay intact.
    let source = r#"
module M where

template T
  with
    primary : Party
    counter : Party
    custodian : Party
  where
    signatory primary

    choice Use : ()
      controller primary, counter
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 2, "expected 2 CPS mutants, got {m:?}");
    for mutant in &m {
        assert!(
            mutant.old_text == "primary" || mutant.old_text == "counter",
            "CPS old_text should be a bare party identifier; got {:?}",
            mutant.old_text
        );
        assert!(
            !mutant.old_text.contains(','),
            "CPS old_text should not include the comma separator; got {:?}",
            mutant.old_text
        );
    }
}

#[test]
fn cps_accepts_module_qualified_party_field() {
    let source = r#"
module M where

template Vault
  with
    owner : M.Party
    custodian : M.Party
  where
    signatory owner

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 1, "expected one CPS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "owner");
    assert_eq!(m[0].new_text, "custodian");
}

#[test]
fn cps_accepts_mixed_unqualified_and_qualified_party_fields() {
    let source = r#"
module M where

template Vault
  with
    owner : Party
    custodian : M.Party
  where
    signatory owner

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 1, "expected one CPS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "owner");
    assert_eq!(m[0].new_text, "custodian");
}

#[test]
fn cps_swaps_qualified_party_controller_to_qualified_party_candidate() {
    let source = r#"
module M where

template Vault
  with
    owner : M.Party
    custodian : M.Party
  where
    signatory owner

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 1, "expected one CPS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "owner");
    assert_eq!(m[0].new_text, "custodian");
}

#[test]
fn cps_collects_qualified_party_candidate_when_party_appears_in_signatory_list() {
    let source = r#"
module M where

template Vault
  with
    owner : M.Party
    custodian : M.Party
  where
    signatory owner, custodian

    choice Use : ()
      controller owner
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(m.len(), 1, "expected one CPS mutant, got {m:?}");
    assert_eq!(m[0].old_text, "owner");
    assert_eq!(m[0].new_text, "custodian");
}

#[test]
fn cps_targets_qualified_party_choice_local_parameter() {
    let source = r#"
module M where

template Escrow
  with
    primary : Party
  where
    signatory primary

    choice Reassign : ()
      with
        actor : M.Party
      controller primary
      do
        return ()
"#;
    let m = mutants_for_slug(source, "CPS");
    assert_eq!(
        m.len(),
        1,
        "expected one CPS mutant (primary -> actor), got {m:?}"
    );
    assert_eq!(m[0].old_text, "primary");
    assert_eq!(m[0].new_text, "actor");
}
