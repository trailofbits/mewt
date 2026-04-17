use mewt::LanguageEngine;
use mewt::languages::sui_move::engine::MoveLanguageEngine;
use mewt::types::{Mutant, Target};
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;

pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
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

pub(crate) fn mutants_for_slug(source: &str, slug: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == slug)
        .collect()
}

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let selected: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == slug).collect();
    assert!(!selected.is_empty(), "expected at least one {slug} mutant");

    let normalize = |text: &str| text.trim().replace('\r', "");

    let mut covered_tokens: HashSet<&str> = HashSet::new();
    let mut unexpected_mutants: Vec<String> = Vec::new();

    for mutant in &selected {
        let matches: Vec<&str> = expected_new_texts
            .iter()
            .copied()
            .filter(|needle| mutant.new_text.contains(needle))
            .collect();

        if matches.is_empty() {
            unexpected_mutants.push(normalize(&mutant.new_text));
        } else {
            for needle in matches {
                covered_tokens.insert(needle);
            }
        }
    }

    assert!(
        unexpected_mutants.is_empty(),
        "found {slug} mutants with unexpected replacements: {unexpected_mutants:?}"
    );

    for expected in expected_new_texts {
        assert!(
            covered_tokens.contains(expected),
            "missing expected {slug} mutant containing: {expected}"
        );
    }
}

#[test]
fn test_basic_sui_move_mutations() {
    let source = r#"module test::m {
    fun demo(x: u64): u64 {
        if (x > 0) { x } else { 0 }
    }
}"#;
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(!mutants.is_empty(), "Should generate mutations");

    let slugs: HashSet<_> = mutants
        .iter()
        .map(|m| m.mutation_slug.chars().take(2).collect::<String>())
        .collect();
    assert!(slugs.len() > 1, "Should generate diverse mutation types");
}

#[test]
fn test_sui_move_mutations_skip_comment_only_lines() {
    let source = r#"module test::m {
    fun demo(x: u64): u64 {
        // keep me
        if (x > 0) { x } else { 0 }
    }
}"#;
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let comment_mutations = mutants
        .iter()
        .filter(|m| m.old_text.trim_start().starts_with("//"))
        .count();

    assert_eq!(
        comment_mutations, 0,
        "Mutations should not target comment-only lines"
    );
}

#[test]
fn test_sui_move_engine_handles_complex_module() {
    let source = r#"module test::m {
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
}"#;
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let result = std::panic::catch_unwind(|| engine.mutate(&target));

    assert!(
        result.is_ok(),
        "Sui Move engine should handle complex modules without panicking"
    );

    if let Ok(mutants) = result {
        assert!(
            mutants.len() > 5,
            "Complex modules should yield many mutations"
        );
    }
}

#[test]
fn test_sui_move_mutations_cover_multiple_lines() {
    let source = r#"module test::m {
    fun demo(x: u64, y: u64): u64 {
        let z = x + y;
        if (z > 0) { z } else { y }
    }
}"#;
    let (_tmp, target) = create_test_target(source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mut lines: HashMap<usize, Vec<String>> = HashMap::new();
    for mutant in &mutants {
        lines
            .entry(mutant.line_offset as usize)
            .or_default()
            .push(mutant.mutation_slug.clone());
    }

    assert!(
        lines.len() > 1,
        "Mutations should touch multiple lines for reasonable coverage"
    );
}

#[test]
fn test_example_file() {
    let source = std::fs::read_to_string("tests/sui_move/examples/hello.move")
        .expect("Failed to read example file");
    let (_tmp, target) = create_test_target(&source);
    let engine = MoveLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        mutants.len() > 10,
        "Example file should generate more than 10 mutants, got {}",
        mutants.len()
    );
}
