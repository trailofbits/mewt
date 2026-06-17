use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;
use mewt::types::Target;

/// Helper to create test target.
pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_extension("rust", "rs", content).into_parts()
}

#[test]
fn rust_common_conformance_checks() {
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"
fn test_func() -> i32 {
    let x = 42;
    if x > 0 {
        return x;
    }
    0
}
"#,
        comment_source: r#"
fn test_func() -> i32 {
    // This is a comment
    let x = 42;
    if x > 0 {
        return x;
    }
    0
}
"#,
        complex_source: r#"
use std::collections::HashMap;

struct Counter {
    value: i32,
}

impl Counter {
    fn new() -> Self {
        Counter { value: 0 }
    }

    fn increment(&mut self) -> i32 {
        self.value += 1;
        self.value
    }

    fn process_message(&self, data: &[u8]) -> Result<i32, String> {
        if data.is_empty() {
            return Err("Empty data".to_string());
        }

        let mut sum = 0;
        for byte in data {
            sum += *byte as i32;
        }

        Ok(sum)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut counter = Counter::new();
    let _ = counter.increment();
    Ok(())
}
"#,
        line_coverage_source: r#"
fn test_func() -> i32 {
    let x = 42;
    let y = x + 1;
    if x > 0 {
        return x;
    }
    y
}
"#,
    };

    let expectations = conformance::CommonConformanceExpectations {
        language_name: "rust",
        min_complex_mutants: 6,
    };

    conformance::run_common_language_checks(
        create_test_target,
        || Box::new(RustLanguageEngine::new()),
        sources,
        expectations,
    );
}

#[test]
fn rust_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/rust/example.rs");
    let (_tmp, target) = create_test_target(&source);
    let mutants = RustLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Rust example file should generate mutants"
    );
}

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = RustLanguageEngine::new();
    utils::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}

fn rust_target_from_source(source: &str) -> Target {
    utils::target_fixture_for_extension("rust", "rs", source).into_target()
}

#[test]
fn rust_mutations_ignore_comment_regions() {
    let source = r#"// if true { assert!(false); }
// let x = 1 + 2;
// if 1 < 2 { let y = 3; }
// foo(10, 20);
// while true { break; }
fn main() {

    let x: i32 = 1 + 2;
    if x > 0 { return; }
}
"#;

    // NOTE: Keep this list in sync with source above.
    // Lines are 0-based and refer to fully-commented lines only.
    let commented_lines: &[usize] = &[0, 1, 2, 3, 4];

    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);

    for m in &mutants {
        let line = m.line_offset as usize;
        assert!(
            !commented_lines.contains(&line),
            "mutated on commented line: slug={} line={}",
            m.mutation_slug,
            line
        );
    }

    // Ensure CR does not double-wrap block-commented content
    let cr_nested = mutants
        .iter()
        .any(|m| m.mutation_slug == "CR" && m.new_text.contains("/* /*"));
    assert!(!cr_nested, "CR should not double-wrap commented content");
}
