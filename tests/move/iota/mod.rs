use std::collections::BTreeSet;

use crate::conformance;
use mewt::languages::r#move::dialect::validate_source_for_dialect;
use mewt::types::config::MoveDialect;

use super::shared;

#[test]
fn move_iota_common_conformance_checks() {
    shared::run_common_conformance_checks("Move/iota", "Move/iota");
}

#[test]
fn move_iota_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/sui_move/example.move");
    let mutants = shared::mutate_source(&source, "Move/iota");

    assert!(
        !mutants.is_empty(),
        "Move/iota example file should generate mutants"
    );
}

#[test]
fn move_iota_baseline_slug_set_matches_sui_currently() {
    let source = r#"module test::m {
    fun demo(a: u64, b: u64, flag: bool): u64 {
        let c = a + b;
        if (!(flag && c > 0)) {
            return 0
        };
        while (c > 10) {
            break;
        };
        c
    }
}"#;

    let sui_slugs: BTreeSet<String> = shared::mutate_source(source, "Move/sui")
        .into_iter()
        .map(|m| m.mutation_slug)
        .collect();
    let iota_slugs: BTreeSet<String> = shared::mutate_source(source, "Move/iota")
        .into_iter()
        .map(|m| m.mutation_slug)
        .collect();

    assert_eq!(
        iota_slugs, sui_slugs,
        "until dialect-specific constraints are introduced, iota and sui should expose the same mutation slugs for baseline sources"
    );
}

#[test]
fn iota_sensitive_marker_is_rejected_for_sui_and_allowed_for_iota() {
    let source = "// @iota_only\nmodule test::m {}";

    assert!(validate_source_for_dialect(source, MoveDialect::Iota).is_ok());
    assert!(validate_source_for_dialect(source, MoveDialect::Sui).is_err());
}
