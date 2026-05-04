use crate::conformance;

use super::shared;

#[test]
fn move_sui_common_conformance_checks() {
    shared::run_common_conformance_checks("Move/sui", "Move/sui");
}

#[test]
fn move_sui_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/sui_move/example.move");
    let mutants = shared::mutate_source(&source, "Move/sui");

    assert!(
        !mutants.is_empty(),
        "Move/sui example file should generate mutants"
    );
}
