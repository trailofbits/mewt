use crate::conformance;
use mewt::languages::r#move::dialect::{MoveDialect, config_for_dialect};

use super::shared;

#[test]
fn move_sui_common_conformance_checks() {
    shared::run_common_conformance_checks("Move/sui", "Move/sui");
}

#[test]
fn move_sui_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/move/example.move");
    let mutants = shared::mutate_source(&source, "Move/sui");

    assert!(
        !mutants.is_empty(),
        "Move/sui example file should generate mutants"
    );
}

#[test]
fn sui_dialect_accepts_package_visibility_construct() {
    let source = r#"module test::m {
    public(package) fun demo() {}
}"#;

    let dialect_config = config_for_dialect(MoveDialect::Sui);
    let tree = mewt::utils::parse_source(source, dialect_config.parser_language());
    assert!(
        tree.is_some(),
        "Sui grammar should accept public(package) visibility"
    );
}
