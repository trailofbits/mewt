use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;
use mewt::types::Target;

/// Helper to create test target.
pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_extension("Go", "go", content).into_parts()
}

#[test]
fn go_common_conformance_checks() {
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"package main

func testFunc() int {
    x := 42
    if x > 0 {
        return x
    }
    return 0
}
"#,
        comment_source: r#"package main

func testFunc() int {
    // This is a comment
    x := 42
    if x > 0 {
        return x
    }
    return 0
}
"#,
        complex_source: r#"package main

import "fmt"

type Counter struct {
    value int
}

func NewCounter() *Counter {
    return &Counter{value: 0}
}

func (c *Counter) Increment() int {
    c.value++
    return c.value
}

func (c *Counter) ProcessMessage(data []byte) (int, error) {
    if len(data) == 0 {
        return 0, fmt.Errorf("empty data")
    }

    sum := 0
    for _, b := range data {
        sum += int(b)
    }

    return sum, nil
}

func main() {
    counter := NewCounter()
    _ = counter.Increment()
}
"#,
        line_coverage_source: r#"package main

func testFunc() int {
    x := 42
    y := x + 1
    if x > 0 {
        return x
    }
    return y
}
"#,
    };

    let expectations = conformance::CommonConformanceExpectations {
        language_name: "Go",
        min_complex_mutants: 6,
    };

    conformance::run_common_language_checks(
        create_test_target,
        || Box::new(GoLanguageEngine::new()),
        sources,
        expectations,
    );
}

#[test]
fn go_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/go/example.go");
    let (_tmp, target) = create_test_target(&source);
    let mutants = GoLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Go example file should generate mutants"
    );
}

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = GoLanguageEngine::new();
    utils::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}
