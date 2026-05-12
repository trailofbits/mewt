use std::collections::BTreeSet;

use crate::conformance;

use super::shared;

#[test]
fn move_aptos_common_conformance_checks() {
    shared::run_common_conformance_checks("Move/aptos", "Move/aptos");
}

#[test]
fn move_aptos_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/move/example.move");
    let mutants = shared::mutate_source(&source, "Move/aptos");

    assert!(
        !mutants.is_empty(),
        "Move/aptos example file should generate mutants"
    );
}

#[test]
fn move_aptos_baseline_slug_set_is_nonempty_and_overlaps_sui() {
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
    let aptos_slugs: BTreeSet<String> = shared::mutate_source(source, "Move/aptos")
        .into_iter()
        .map(|m| m.mutation_slug)
        .collect();

    assert!(
        !aptos_slugs.is_empty(),
        "Move/aptos should expose at least one mutation slug"
    );
    assert!(
        aptos_slugs.intersection(&sui_slugs).next().is_some(),
        "Move/aptos and Move/sui should share at least some mutation slugs"
    );
}
