use mewt::LanguageEngine;
use mewt::languages::r#move::dialect::MoveDialect;
use mewt::languages::r#move::engine::MoveDialectEngine;

use crate::utils;

#[test]
fn acq_is_only_exposed_for_aptos_move() {
    let sui = MoveDialectEngine::new(MoveDialect::Sui);
    let iota = MoveDialectEngine::new(MoveDialect::Iota);
    let aptos = MoveDialectEngine::new(MoveDialect::Aptos);

    assert!(!sui.get_mutations().iter().any(|m| m.slug == "ACQ"));
    assert!(!iota.get_mutations().iter().any(|m| m.slug == "ACQ"));
    assert!(aptos.get_mutations().iter().any(|m| m.slug == "ACQ"));
}

#[test]
fn acq_removes_aptos_acquires_clause() {
    let source = r#"module 0x1::m {
    struct Store has key { value: u64 }

    public fun read(addr: address): u64 acquires Store {
        borrow_global<Store>(addr).value
    }
}
"#;

    let fixture = utils::target_fixture_for_extension("move/aptos", "move", source);
    let target = fixture.into_target();
    let engine = MoveDialectEngine::new(MoveDialect::Aptos);
    let mutants: Vec<_> = engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "ACQ")
        .collect();

    assert_eq!(mutants.len(), 1, "expected one ACQ mutant: {mutants:?}");
    assert_eq!(mutants[0].old_text.trim(), "acquires Store");
    assert_eq!(mutants[0].new_text, "");
}

#[test]
fn aptos_catalog_filters_unsupported_negation_removal() {
    let aptos = MoveDialectEngine::new(MoveDialect::Aptos);
    assert!(!aptos.get_mutations().iter().any(|m| m.slug == "NR"));
}
