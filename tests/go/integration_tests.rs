use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;
use mewt::types::Target;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tempfile::tempdir;

/// Helper to create test target
pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
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
    let ast_mutants = ast_engine.mutate(&target);

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
    let ast_mutants = ast_engine.mutate(&target);

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
    let ast_result = std::panic::catch_unwind(|| ast_engine.mutate(&target));

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
    let ast_mutants = ast_engine.mutate(&target);

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

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = GoLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let selected: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == slug).collect();
    assert!(!selected.is_empty(), "expected at least one {slug} mutant");
    assert!(
        mutants
            .iter()
            .filter(|m| expected_new_texts
                .iter()
                .any(|text| m.new_text.contains(text)))
            .all(|m| m.mutation_slug == slug),
        "expected snippets should only come from {slug} mutants"
    );

    for expected in expected_new_texts {
        assert!(
            selected.iter().any(|m| m.new_text.contains(expected)),
            "missing expected {slug} mutant containing: {expected}"
        );
    }
}

#[test]
fn er_and_cr_cover_all_simple_statements() {
    // Regression test for .todo/c8b410d5: Go's ER/CR arms previously targeted
    // only expression_statement / return_statement / if_statement /
    // for_statement, leaving short-var declarations, assignments (plain and
    // compound), and inc/dec statements without any ER/CR coverage. Each kind
    // below must now produce at least one ER and one CR mutant.
    let source = r#"package main

func f() {
    x := 0
    x = 1
    x += 1
    x++
    x--
    _ = x
}
"#;
    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new().mutate(&target);

    // For each statement form, the `old_text` of an ER/CR mutant should
    // begin with a distinctive prefix. Collect (old_text_prefix, label)
    // pairs and assert that every form has at least one ER and one CR.
    let cases: &[(&str, &str)] = &[
        ("x :=", "short_var_declaration"),
        ("x = 1", "assignment_statement (plain)"),
        ("x +=", "assignment_statement (compound)"),
        ("x++", "inc_statement"),
        ("x--", "dec_statement"),
    ];

    for (prefix, label) in cases {
        let has_er = mutants
            .iter()
            .any(|m| m.mutation_slug == "ER" && m.old_text.trim_start().starts_with(prefix));
        let has_cr = mutants
            .iter()
            .any(|m| m.mutation_slug == "CR" && m.old_text.trim_start().starts_with(prefix));
        assert!(
            has_er,
            "expected an ER mutant for {} (prefix {:?}); got mutants: {:?}",
            label,
            prefix,
            mutants
                .iter()
                .map(|m| (m.mutation_slug.as_str(), m.old_text.as_str()))
                .collect::<Vec<_>>()
        );
        assert!(
            has_cr,
            "expected a CR mutant for {} (prefix {:?}); got mutants: {:?}",
            label,
            prefix,
            mutants
                .iter()
                .map(|m| (m.mutation_slug.as_str(), m.old_text.as_str()))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn compound_assignment_slugs_produce_mutants() {
    // Regression test for .todo/a3c12f04: AAOS/BAOS/SAOS were explicitly
    // opted out as "not applicable to Go", but Go has all of these compound
    // assignment operators (including the Go-specific `&^=`). The slugs now
    // fire against `assignment_statement` nodes.
    let source = r#"package main

func f() {
    x := 0
    x += 1
    x -= 1
    x &= 1
    x |= 1
    x &^= 1
    x <<= 1
    x >>= 1
    _ = x
}
"#;
    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new().mutate(&target);
    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    for slug in ["AAOS", "BAOS", "SAOS"] {
        assert!(
            slugs.contains(slug),
            "expected slug {} to produce at least one mutant; got slugs: {:?}",
            slug,
            slugs
        );
    }
    // Specifically verify Go's bit-clear operator `&^=` generates BAOS mutants
    assert!(
        mutants
            .iter()
            .any(|m| m.mutation_slug == "BAOS" && m.old_text == "&^="),
        "expected a BAOS mutant with old_text `&^=`"
    );
}

#[test]
fn test_negation_removal() {
    let source = r#"package main

func check(ok bool) bool {
    if !ok {
        return false
    }
    return !(ok && true)
}
"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = GoLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert_eq!(nr.len(), 2, "Should generate exactly 2 NR mutations");
    assert!(
        nr.iter().any(|m| m.old_text == "!ok" && m.new_text == "ok"),
        "NR should replace !ok with ok: {nr:?}"
    );
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(ok && true)" && m.new_text == "(ok && true)"),
        "NR should replace !(ok && true) with (ok && true): {nr:?}"
    );
}

#[test]
fn test_negation_removal_ignores_other_unary_ops() {
    let source = r#"package main

func f(x int) int {
    return -x
}
"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = GoLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert!(nr.is_empty(), "NR should not trigger on - unary operator");
}

#[test]
fn test_negation_removal_in_comment_ignored() {
    let source = r#"package main

// if !ok { return }
/* !flag */
func f() {}
"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = GoLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert!(
        nr.is_empty(),
        "NR should not generate mutations inside comments"
    );
}

#[test]
fn compound_assignment_slug_tests_are_present() {
    let engine = GoLanguageEngine::new();
    let compound_slugs: Vec<&str> = engine
        .get_mutations()
        .iter()
        .map(|m| m.slug)
        .filter(|slug| *slug != "AOS" && slug.ends_with("AOS"))
        .collect();

    for slug in compound_slugs {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("go")
            .join("mutations")
            .join(format!("{}.rs", slug.to_lowercase()));
        assert!(
            path.exists(),
            "missing per-slug test file for {slug}: {path:?}"
        );
    }
}
