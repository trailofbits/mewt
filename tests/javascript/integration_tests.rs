use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::types::Target;
use std::collections::HashSet;

pub(crate) fn create_test_target(content: &str, filename: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_filename("JavaScript", filename, content).into_parts()
}

#[test]
fn test_basic_javascript_mutations() {
    let source = r#"
function testFunc() {
    const x = 42;
    if (x > 0) {
        return x;
    }
    return 0;
}
"#;
    let (_temp_dir, target) = create_test_target(source, "test.js");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(!mutants.is_empty(), "Should generate mutations");

    let slugs: HashSet<_> = mutants.iter().map(|m| &m.mutation_slug[..2]).collect();
    assert!(slugs.len() > 1, "Should generate diverse mutation types");
}

#[test]
fn typescript_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/javascript/example.ts");
    let (_tmp, target) = create_test_target(&source, "example.ts");
    let mutants = JavaScriptLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "TypeScript example file should generate mutants"
    );
}

#[test]
fn jsx_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/javascript/example.jsx");
    let (_tmp, target) = create_test_target(&source, "example.jsx");
    let mutants = JavaScriptLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "JSX example file should generate mutants"
    );
}

#[test]
fn tsx_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/javascript/example.tsx");
    let (_tmp, target) = create_test_target(&source, "example.tsx");
    let mutants = JavaScriptLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "TSX example file should generate mutants"
    );
}

#[test]
fn test_operator_mutations() {
    let source = r#"
function calc(a, b) {
    const sum = a + b;
    const diff = a - b;
    const prod = a * b;
    return sum && diff || prod;
}
"#;
    let (_temp_dir, target) = create_test_target(source, "test.js");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let aos_count = mutants
        .iter()
        .filter(|m| m.mutation_slug.starts_with("AOS"))
        .count();
    let los_count = mutants
        .iter()
        .filter(|m| m.mutation_slug.starts_with("LOS"))
        .count();

    assert!(
        aos_count > 0,
        "Should generate arithmetic operator mutations"
    );
    assert!(los_count > 0, "Should generate logical operator mutations");
}

#[test]
fn javascript_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/javascript/example.js");
    let (_tmp, target) = create_test_target(&source, "example.js");
    let mutants = JavaScriptLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "JavaScript example file should generate mutants"
    );
}

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    filename: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source, filename);
    let engine = JavaScriptLanguageEngine::new();
    utils::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}
