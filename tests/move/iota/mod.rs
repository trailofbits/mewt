use std::collections::BTreeSet;

use crate::conformance;

use super::shared;

#[test]
fn move_iota_common_conformance_checks() {
    shared::run_common_conformance_checks("move/iota", "move/iota");
}

#[test]
fn move_iota_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/move/example.move");
    let mutants = shared::mutate_source(&source, "move/iota");

    assert!(
        !mutants.is_empty(),
        "move/iota example file should generate mutants"
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

    let sui_slugs: BTreeSet<String> = shared::mutate_source(source, "move/sui")
        .into_iter()
        .map(|m| m.mutation_slug)
        .collect();
    let iota_slugs: BTreeSet<String> = shared::mutate_source(source, "move/iota")
        .into_iter()
        .map(|m| m.mutation_slug)
        .collect();

    assert_eq!(
        iota_slugs, sui_slugs,
        "until dialect-specific constraints are introduced, iota and sui should expose the same mutation slugs for baseline sources"
    );
}
