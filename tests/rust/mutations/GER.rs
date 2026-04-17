use crate::rust::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;
use mewt::types::Mutant;

fn ger_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp_dir, target) = create_test_target(source);
    RustLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "GER")
        .collect()
}

#[test]
fn ger_supports_unit_and_basic_scalar_returns() {
    let source = r#"
fn unit_fn() {
    ping();
}

fn bool_fn() -> bool {
    ping();
    false
}

fn int_fn() -> i32 {
    ping();
    1
}

fn float_fn() -> f64 {
    ping();
    1.0
}
"#;
    let ger = ger_mutants(source);

    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return;"),
        "expected `ping();` -> `return;` for unit function: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return false;"),
        "expected `ping();` -> `return false;` for bool function: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return 0;"),
        "expected `ping();` -> `return 0;` for int function: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return 0.0;"),
        "expected `ping();` -> `return 0.0;` for float function: {ger:?}"
    );
}

#[test]
fn ger_skips_unsupported_return_types() {
    let source = r#"
fn string_fn() -> String {
    let x = 1;
    format!("{x}")
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.is_empty(),
        "GER should skip unsupported Rust return types: {ger:?}"
    );
}

#[test]
fn ger_does_not_target_return_statements_or_closures() {
    let source = r#"
fn outer() -> bool {
    let f = || {
        ping();
        true
    };
    if true {
        ping();
    }
    return f();
}
"#;
    let ger = ger_mutants(source);

    assert!(
        ger.iter().any(
            |m| m.old_text.trim_start().starts_with("if true") && m.new_text == "return false;"
        ),
        "expected outer if-expression to be mutated: {ger:?}"
    );
    assert!(
        !ger.iter().any(|m| m.old_text.contains("return f();")),
        "GER should not target existing return statements: {ger:?}"
    );
    assert!(
        !ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return false;"),
        "GER should not mutate statements inside closures: {ger:?}"
    );
}

#[test]
fn ger_skips_result_and_option() {
    let source = r#"
fn result_fn() -> Result<i32, String> {
    ping();
    Ok(1)
}

fn option_fn() -> Option<i32> {
    ping();
    Some(1)
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.is_empty(),
        "GER should skip Result and Option return types: {ger:?}"
    );
}

#[test]
fn ger_skips_str_ref() {
    let source = r#"
fn get_name() -> &'static str {
    ping();
    "hello"
}
"#;
    let ger = ger_mutants(source);
    assert!(ger.is_empty(), "GER should skip &str return type: {ger:?}");
}

#[test]
fn ger_works_in_impl_methods() {
    let source = r#"
struct Foo;
impl Foo {
    fn bar(&self) -> i32 {
        ping();
        42
    }
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return 0;"),
        "GER should work inside impl methods: {ger:?}"
    );
}

#[test]
fn ger_picks_correct_return_type_across_functions() {
    let source = r#"
fn bool_fn() -> bool {
    ping();
    true
}

fn int_fn() -> i32 {
    ping();
    42
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return false;"),
        "bool_fn should become return false: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return 0;"),
        "int_fn should become return 0: {ger:?}"
    );
}

#[test]
fn ger_on_last_statement_before_return() {
    let source = r#"
fn f() -> i32 {
    ping();
    return 42;
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return 0;"),
        "GER should replace statement before return: {ger:?}"
    );
}
