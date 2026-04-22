use crate::javascript::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::types::Mutant;

fn ger_mutants(source: &str, filename: &str) -> Vec<Mutant> {
    let (_tmp_dir, target) = create_test_target(source, filename);
    JavaScriptLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "GER")
        .collect()
}

#[test]
fn ger_js_bare_returns() {
    let source = r#"
function unitFn() {
    doThing();
}

const arrowUnit = () => {
    doThing();
}
"#;
    let ger = ger_mutants(source, "test.js");
    assert!(
        ger.iter()
            .filter(|m| m.old_text.trim() == "doThing();")
            .count()
            >= 2,
        "expected GER on both function and arrow-function: {ger:?}"
    );
    assert!(
        ger.iter().all(|m| m.new_text == "return;"),
        "JavaScript GER should always emit bare returns: {ger:?}"
    );
}

#[test]
fn ger_ts_type_aware_returns() {
    let source = r#"
function boolFn(): boolean {
    doThing();
    return false;
}

function stringFn(): string {
    doThing();
    return `${1}`;
}

function numberFn(): number {
    doThing();
    return 1;
}

function voidFn(): void {
    doThing();
}
"#;
    let ger = ger_mutants(source, "test.ts");
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "doThing();" && m.new_text == "return false;"),
        "expected return false for boolean: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "doThing();" && m.new_text == "return \"\";"),
        "expected return \"\" for string: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "doThing();" && m.new_text == "return 0;"),
        "expected return 0 for number: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "doThing();" && m.new_text == "return;"),
        "expected return; for void: {ger:?}"
    );
}

#[test]
fn ger_skips_unsupported_ts_types() {
    let source = r#"
type User = { name: string };

function buildUser(): User {
    if (true) {
        doThing();
    }
    return { name: "a" };
}
"#;
    let ger = ger_mutants(source, "test.ts");
    assert!(
        ger.is_empty(),
        "GER should skip unsupported TypeScript return types: {ger:?}"
    );
}

#[test]
fn ger_does_not_target_existing_returns() {
    let js_ger = ger_mutants(
        r#"
function unitFn() {
    doThing();
    return value;
}
"#,
        "test.js",
    );
    assert!(
        !js_ger
            .iter()
            .any(|m| m.old_text.trim_start().starts_with("return ")),
        "GER should not target existing JS return statements: {js_ger:?}"
    );

    let ts_ger = ger_mutants(
        r#"
function boolFn(): boolean {
    doThing();
    return false;
}
"#,
        "test.ts",
    );
    assert!(
        !ts_ger
            .iter()
            .any(|m| m.old_text.trim_start().starts_with("return ")),
        "GER should not target existing TS return statements: {ts_ger:?}"
    );
}

#[test]
fn ger_class_method() {
    let ger = ger_mutants(
        r#"
class Foo {
    bar(): number {
        doThing();
        return 42;
    }
}
"#,
        "test.ts",
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "doThing();" && m.new_text == "return 0;"),
        "GER should work inside class methods: {ger:?}"
    );
}

#[test]
fn ger_nested_in_if_block() {
    let ger = ger_mutants(
        r#"
function f() {
    if (true) {
        doThing();
    }
}
"#,
        "test.js",
    );
    assert!(!ger.is_empty(), "GER should fire inside if blocks: {ger:?}");
}

#[test]
fn ger_picks_correct_return_type_across_functions() {
    let ger = ger_mutants(
        r#"
function boolFn(): boolean {
    doThing();
    return true;
}

function numFn(): number {
    doThing();
    return 42;
}
"#,
        "test.ts",
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "doThing();" && m.new_text == "return false;"),
        "boolFn should become return false: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "doThing();" && m.new_text == "return 0;"),
        "numFn should become return 0: {ger:?}"
    );
}

#[test]
fn ger_generator_function() {
    let ger = ger_mutants(
        r#"
function* gen() {
    yield 1;
    yield 2;
}
"#,
        "test.js",
    );
    if !ger.is_empty() {
        assert!(
            ger.iter().all(|m| m.new_text == "return;"),
            "GER in JS generators should use bare return: {ger:?}"
        );
    }
}
