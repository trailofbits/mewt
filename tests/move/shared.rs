use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::r#move::engine::MoveLanguageEngine;
use mewt::types::{Mutant, Target};

pub(crate) fn create_test_target(content: &str, language: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_extension(language, "move", content).into_parts()
}

pub(crate) fn mutate_source(source: &str, language: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source, language);
    let engine = MoveLanguageEngine::new();
    engine.mutate(&target)
}

pub(crate) fn run_common_conformance_checks(language: &str, language_name: &str) {
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
        language_name,
        min_complex_mutants: 6,
    };

    conformance::run_common_language_checks(
        |content| create_test_target(content, language),
        || Box::new(MoveLanguageEngine::new()),
        sources,
        expectations,
    );
}
