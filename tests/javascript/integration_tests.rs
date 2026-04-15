use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::types::Target;
use std::collections::HashSet;
use tempfile::tempdir;

pub(crate) fn create_test_target(content: &str, filename: &str) -> (tempfile::TempDir, Target) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join(filename);
    std::fs::write(&file_path, content).expect("Failed to write test file");
    let target = Target {
        id: 1,
        path: file_path,
        file_hash: mewt::types::Hash::digest(content.to_string()),
        text: content.to_string(),
        language: "JavaScript".to_string(),
    };
    (temp_dir, target)
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
fn test_typescript_support() {
    let source = r#"
interface User {
    name: string;
    age: number;
}

function greet(user: User): string {
    if (user.age > 18) {
        return `Hello, ${user.name}!`;
    }
    return "Hello!";
}
"#;
    let (_temp_dir, target) = create_test_target(source, "test.ts");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Should generate mutations for TypeScript"
    );
}

#[test]
fn test_jsx_support() {
    let source = r#"
function Welcome(props) {
    if (props.show) {
        return <h1>Hello, {props.name}</h1>;
    }
    return null;
}
"#;
    let (_temp_dir, target) = create_test_target(source, "test.jsx");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(!mutants.is_empty(), "Should generate mutations for JSX");
}

#[test]
fn test_tsx_support() {
    let source = r#"
import type { FC } from "react";

const Button: FC<{ label: string; onClick(): void }> = ({ label, onClick }) => {
    if (onClick) {
        return <button onClick={onClick}>{label}</button>;
    }
    return null;
};
"#;
    let (_temp_dir, target) = create_test_target(source, "test.tsx");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Should generate mutations for TSX files"
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
