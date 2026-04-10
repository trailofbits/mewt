use mewt::LanguageEngine;
use mewt::languages::sui_move::engine::MoveLanguageEngine;
use mewt::types::Target;
use std::collections::HashSet;
use tempfile::tempdir;

fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.move");
    std::fs::write(&file_path, content).expect("Failed to write test file");
    let target = Target {
        id: 1,
        path: file_path,
        file_hash: mewt::types::Hash::digest(content.to_string()),
        text: content.to_string(),
        language: "SuiMove".to_string(),
    };
    (temp_dir, target)
}

#[test]
fn test_generates_mutations() {
    let source = r#"module test::m {
    fun add(a: u64, b: u64): u64 {
        a + b
    }
}"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);
    assert!(
        !mutants.is_empty(),
        "Should generate mutations for Move code"
    );
}

#[test]
fn test_if_condition_mutations() {
    let source = r#"module test::m {
    fun check(x: u64): bool {
        if (x > 0) { true } else { false }
    }
}"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let if_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "IF" || m.mutation_slug == "IT")
        .collect();
    assert!(!if_mutants.is_empty(), "Should generate IF/IT mutations");
}

#[test]
fn test_bool_literal_mutations() {
    let source = r#"module test::m {
    fun always_true(): bool { true }
    fun always_false(): bool { false }
}"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let bl_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "BL").collect();
    assert!(
        !bl_mutants.is_empty(),
        "Should generate BL mutations for boolean literals"
    );
}

#[test]
fn test_arithmetic_operator_mutations() {
    let source = r#"module test::m {
    fun math(a: u64, b: u64): u64 {
        let x = a + b;
        let y = a - b;
        let z = a * b;
        x
    }
}"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let aos_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "AOS")
        .collect();
    assert!(
        !aos_mutants.is_empty(),
        "Should generate AOS mutations for arithmetic operators"
    );
}

#[test]
fn test_while_loop_mutations() {
    let source = r#"module test::m {
    fun count(n: u64): u64 {
        let mut i = 0u64;
        while (i < n) { i = i + 1; };
        i
    }
}"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let wf_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "WF").collect();
    assert!(
        !wf_mutants.is_empty(),
        "Should generate WF mutations for while loops"
    );
}

#[test]
fn test_error_replacement_avoids_existing_aborts() {
    let source = r#"module test::m {
    fun safe(b: u64): u64 {
        if (b == 0) { abort 0 };
        42 / b
    }
}"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // ER mutants should not replace existing abort statements
    let er_abort_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "ER" && m.old_text.contains("abort "))
        .collect();
    assert_eq!(
        er_abort_mutants.len(),
        0,
        "ER should not replace existing abort statements"
    );
}

#[test]
fn test_diverse_mutation_types() {
    let source = r#"module test::m {
    fun complex(a: u64, b: u64): u64 {
        let mut result = 0u64;
        if (a > b) {
            result = a - b;
        } else {
            result = b - a;
        };
        while (result > 10) {
            result = result / 2;
        };
        result
    }
}"#;
    let (_temp_dir, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    println!("Generated mutation types: {slugs:?}");

    assert!(
        slugs.len() >= 3,
        "Should generate at least 3 distinct mutation types for complex code, got: {slugs:?}"
    );
}

#[test]
fn test_example_file() {
    let source = std::fs::read_to_string("tests/sui_move/examples/hello.move")
        .expect("Failed to read example file");
    let (_temp_dir, target) = create_test_target(&source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    println!("Example file generated {} mutants", mutants.len());
    assert!(
        mutants.len() > 10,
        "Example file should generate more than 10 mutants, got {}",
        mutants.len()
    );
}
