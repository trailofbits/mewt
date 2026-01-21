use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;
use mewt::types::Target;
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;

/// Helper to create test target
fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.go");
    std::fs::write(&file_path, content).expect("Failed to write test file");
    let target = Target {
        id: 1,
        path: file_path,
        file_hash: mewt::types::Hash::digest(content.to_string()),
        text: content.to_string(),
        language: "Go".to_string(),
    };
    (temp_dir, target)
}

#[test]
fn test_mutation_count_comparison() {
    let source = r#"package main

func testFunc() int {
    x := 42
    if x > 0 {
        return x
    }
    return 0
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    // Get AST mutations
    let ast_engine = GoLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    println!("AST mutations: {}", ast_mutants.len());

    // AST should generate reasonable number of mutations
    assert!(
        !ast_mutants.is_empty(),
        "AST should generate some mutations"
    );

    // Check mutation types
    let ast_slugs: HashSet<_> = ast_mutants
        .iter()
        .map(|m| m.mutation_slug.chars().take(2).collect::<String>())
        .collect();

    println!("AST mutation types: {ast_slugs:?}");

    // Should generate diverse mutation types
    assert!(
        ast_slugs.len() > 1,
        "AST should generate diverse mutation types"
    );
}

#[test]
fn test_mutation_quality_comparison() {
    let source = r#"package main

func testFunc() int {
    // This is a comment
    x := 42
    if x > 0 {
        return x
    }
    return 0
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    // Get AST mutations
    let ast_engine = GoLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    // Check comment handling (checking old_text for comment patterns)
    let ast_comment_mutations = ast_mutants
        .iter()
        .filter(|m| m.old_text.trim().starts_with("//"))
        .count();

    println!("AST comment mutations: {ast_comment_mutations}");

    // AST should avoid mutating comment-only lines
    assert_eq!(
        ast_comment_mutations, 0,
        "AST should not mutate comment-only lines"
    );
}

#[test]
fn test_complex_code_handling() {
    let source = r#"package main

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
    result := counter.Increment()
    fmt.Println("Result:", result)
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    // Test that AST system can handle complex Go code
    let ast_engine = GoLanguageEngine::new();
    let ast_result = std::panic::catch_unwind(|| ast_engine.apply_all_mutations(&target));

    assert!(
        ast_result.is_ok(),
        "AST system should handle complex code without panicking"
    );

    if let Ok(ast_mutants) = ast_result {
        println!("Complex code - AST mutations: {}", ast_mutants.len());

        // Should generate substantial mutations for complex code
        assert!(
            ast_mutants.len() > 5,
            "AST should generate substantial mutations for complex code"
        );
    }
}

#[test]
fn test_mutation_overlap_analysis() {
    let source = r#"package main

func testFunc() int {
    x := 42
    y := x + 1
    if x > 0 {
        return x
    }
    return y
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    let ast_engine = GoLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    // Analyze which lines are affected by mutations
    let mut ast_lines: HashMap<usize, Vec<String>> = HashMap::new();

    for mutant in &ast_mutants {
        ast_lines
            .entry(mutant.line_offset as usize)
            .or_default()
            .push(mutant.mutation_slug.clone());
    }

    println!("AST mutations by line: {ast_lines:?}");

    // Should affect multiple lines for decent coverage
    assert!(
        ast_lines.len() > 1,
        "AST mutations should affect multiple lines"
    );
}
