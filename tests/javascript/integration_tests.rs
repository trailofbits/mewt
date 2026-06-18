use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::types::Target;

pub(crate) fn create_test_target(content: &str, filename: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_filename("javascript", filename, content).into_parts()
}

#[test]
fn javascript_common_conformance_checks() {
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"
function testFunc() {
    const x = 42;
    if (x > 0) {
        return x;
    }
    return 0;
}
"#,
        comment_source: r#"
function testFunc() {
    // This is a comment
    const x = 42;
    if (x > 0) {
        return x;
    }
    return 0;
}
"#,
        complex_source: r#"
class Counter {
    constructor() {
        this.value = 0;
    }

    increment() {
        this.value += 1;
        return this.value;
    }

    process(values) {
        if (!values || values.length === 0) {
            throw new Error("empty values");
        }

        let sum = 0;
        for (const value of values) {
            sum += value;
        }

        return sum;
    }
}

function main() {
    const counter = new Counter();
    return counter.increment();
}
"#,
        line_coverage_source: r#"
function testFunc() {
    const x = 42;
    const y = x + 1;
    if (x > 0) {
        return x;
    }
    return y;
}
"#,
    };

    let expectations = conformance::CommonConformanceExpectations {
        language_name: "javascript",
        min_complex_mutants: 6,
    };

    conformance::run_common_language_checks(
        |source| create_test_target(source, "test.js"),
        || Box::new(JavaScriptLanguageEngine::new()),
        sources,
        expectations,
    );
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
