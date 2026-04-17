use crate::common;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::types::Target;
use std::collections::HashSet;

pub(crate) fn create_test_target(content: &str, filename: &str) -> (tempfile::TempDir, Target) {
    common::target_fixture_for_filename("JavaScript", filename, content).into_parts()
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
    common::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}
