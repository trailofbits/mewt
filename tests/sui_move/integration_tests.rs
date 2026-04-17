use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::sui_move::engine::MoveLanguageEngine;
use mewt::types::{Mutant, Target};

pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_extension("SuiMove", "move", content).into_parts()
}

pub(crate) fn mutants_for_slug(source: &str, slug: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    utils::mutants_for_slug(&engine, &target, slug)
}

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    utils::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}

#[test]
fn sui_move_common_conformance_checks() {
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"module test::m {
    fun demo(x: u64): u64 {
        if (x > 0) { x } else { 0 }
    }
}"#,
        comment_source: r#"module test::m {
    fun demo(x: u64): u64 {
        // keep me
        if (x > 0) { x } else { 0 }
    }
}"#,
        complex_source: r#"module test::m {
    public fun process(a: u64, b: u64, flag: bool): u64 {
        let mut result = if (flag) { a + b } else { a - b };

        if (!(result > 0)) {
            result = 1;
        };

        while (result > 10) {
            result = result / 2;
        };

        result
    }
}"#,
        line_coverage_source: r#"module test::m {
    fun demo(x: u64, y: u64): u64 {
        let z = x + y;
        if (z > 0) { z } else { y }
    }
}"#,
    };

    let expectations = conformance::CommonConformanceExpectations {
        language_name: "SuiMove",
        min_complex_mutants: 6,
    };

    conformance::run_common_language_checks(
        create_test_target,
        || Box::new(MoveLanguageEngine::new()),
        sources,
        expectations,
    );
}

#[test]
fn sui_move_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/sui_move/example.move");
    let (_tmp, target) = create_test_target(&source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Sui Move example file should generate mutants"
    );
}
