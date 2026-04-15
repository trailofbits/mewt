use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::types::Target;
use std::collections::HashSet;
use tempfile::tempdir;

fn create_test_target(content: &str, filename: &str) -> (tempfile::TempDir, Target) {
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
fn test_typescript_generics_not_mutated() {
    let source = r#"
// TypeScript generics should NOT be mutated
const emitter = module.get<EventEmitter2>(EventEmitter2);
const result = foo<string, number>(arg1, arg2);

function generic<T>(value: T): T {
    return value;
}

// Real comparisons SHOULD be mutated
if (a < b && c > d) {
    return true;
}

const max = x >= y ? x : y;
"#;
    let (_temp_dir, target) = create_test_target(source, "test.ts");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Filter to just COS mutations
    let cos_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug.starts_with("COS"))
        .collect();

    // Should have COS mutations (from the actual comparison operators)
    assert!(
        !cos_mutants.is_empty(),
        "Should generate COS mutations for real comparison operators"
    );

    // Verify no mutations contain "get<", "foo<", or "generic<"
    // (these would indicate mutations of TypeScript generics)
    for mutant in &cos_mutants {
        assert!(
            !mutant.new_text.contains("get<")
                && !mutant.new_text.contains("get==")
                && !mutant.new_text.contains("get!=")
                && !mutant.new_text.contains("get<=")
                && !mutant.new_text.contains("get>=")
                && !mutant.new_text.contains("foo<")
                && !mutant.new_text.contains("foo==")
                && !mutant.new_text.contains("foo!=")
                && !mutant.new_text.contains("foo<=")
                && !mutant.new_text.contains("foo>=")
                && !mutant.new_text.contains("generic<")
                && !mutant.new_text.contains("generic==")
                && !mutant.new_text.contains("generic!=")
                && !mutant.new_text.contains("generic<=")
                && !mutant.new_text.contains("generic>="),
            "COS mutation should not mutate TypeScript generic brackets: {}",
            mutant.new_text
        );
    }

    // Verify we have mutations for the actual comparison operators
    // (The old_text will just be the operator, not the full expression)
    let has_less_than_mutation = cos_mutants.iter().any(|m| m.old_text == "<");
    let has_greater_than_mutation = cos_mutants.iter().any(|m| m.old_text == ">");
    let has_gte_mutation = cos_mutants.iter().any(|m| m.old_text == ">=");

    assert!(
        has_less_than_mutation && has_greater_than_mutation && has_gte_mutation,
        "Should mutate actual comparison operators (<, >, >=) in conditions"
    );
}

#[test]
fn test_tsx_jsx_and_generics_not_mutated() {
    let source = r#"
// TSX with both JSX elements and TypeScript generics
function App<T>(props: { value: T }) {
    return <div>Hello</div>;
}

const result = foo<string, number>(arg1, arg2);
const element = <Component prop="value" />;

// Real comparisons SHOULD be mutated
if (a < b && c > d) {
    return true;
}
"#;
    let (_temp_dir, target) = create_test_target(source, "test.tsx");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Filter to just COS mutations
    let cos_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug.starts_with("COS"))
        .collect();

    // Should have COS mutations (from the actual comparison operators)
    assert!(
        !cos_mutants.is_empty(),
        "Should generate COS mutations for real comparison operators"
    );

    // Verify no mutations contain JSX tags or generic brackets
    for mutant in &cos_mutants {
        let text = &mutant.new_text;
        assert!(
            !text.contains("<div")
                && !text.contains("</div")
                && !text.contains("<Component")
                && !text.contains("App<")
                && !text.contains("foo<")
                && !text.contains("foo==")
                && !text.contains("foo!=")
                && !text.contains("App==")
                && !text.contains("App!="),
            "COS mutation should not mutate TSX JSX elements or TypeScript generics: {}",
            text
        );
    }

    // Verify we have mutations for the actual comparison operators
    let has_comparison_mutations = cos_mutants
        .iter()
        .any(|m| m.old_text == "<" || m.old_text == ">" || m.old_text == ">=");

    assert!(
        has_comparison_mutations,
        "Should mutate actual comparison operators in TSX files"
    );
}

#[test]
fn compound_assignment_slugs_produce_mutants() {
    // Regression test for .todo/a3c12f04: AAOS/BAOS/SAOS were wired to
    // `binary_expression`, but compound assignment in tree-sitter-javascript
    // parses as `augmented_assignment_expression`. The slugs silently emitted
    // zero mutants.
    let source = r#"
function f() {
    let x = 0;
    x += 1;
    x -= 1;
    x **= 2;
    x &= 1;
    x |= 1;
    x <<= 1;
    x >>= 1;
    x >>>= 1;
}
"#;
    let (_tmp, target) = create_test_target(source, "test.js");
    let mutants = JavaScriptLanguageEngine::new().mutate(&target);
    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    for slug in ["AAOS", "BAOS", "SAOS"] {
        assert!(
            slugs.contains(slug),
            "expected slug {} to produce at least one mutant; got slugs: {:?}",
            slug,
            slugs
        );
    }
    // Verify JS-specific operators are covered
    assert!(
        mutants
            .iter()
            .any(|m| m.mutation_slug == "AAOS" && m.old_text == "**="),
        "expected an AAOS mutant with old_text `**=`"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.mutation_slug == "SAOS" && m.old_text == ">>>="),
        "expected a SAOS mutant with old_text `>>>=`"
    );
}

#[test]
fn test_negation_removal() {
    let source = r#"
function check(flag) {
    if (!flag) {
        throw new Error("bad");
    }
    return !(flag && true);
}
"#;
    let (_dir, target) = create_test_target(source, "test.js");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert_eq!(nr.len(), 2, "Should generate exactly 2 NR mutations");
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!flag" && m.new_text == "flag"),
        "NR should replace !flag with flag: {nr:?}"
    );
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(flag && true)" && m.new_text == "(flag && true)"),
        "NR should replace !(flag && true) with (flag && true): {nr:?}"
    );
}

#[test]
fn test_negation_removal_ignores_other_unary_ops() {
    let source = r#"
function f(x) {
    return -x;
}
"#;
    let (_dir, target) = create_test_target(source, "test.js");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert!(nr.is_empty(), "NR should not trigger on - unary operator");
}

#[test]
fn test_negation_removal_in_comment_ignored() {
    let source = r#"
// if (!flag) { throw new Error(); }
/* !x */
function f() {}
"#;
    let (_dir, target) = create_test_target(source, "test.js");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();

    assert!(
        nr.is_empty(),
        "NR should not generate mutations inside comments"
    );
}
