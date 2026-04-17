use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;
use mewt::types::Mutant;

fn ger_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp_dir, target) = create_test_target(source);
    GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "GER")
        .collect()
}

#[test]
fn ger_supports_basic_go_returns() {
    let source = r#"package main

func unitFn() {
    ping()
}

func tupleFn() (int, error) {
    ping()
    return x, nil
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping()" && m.new_text == "return"),
        "expected `ping()` -> `return` for no-result function: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping()" && m.new_text == "return 0, nil"),
        "expected `ping()` -> `return 0, nil` for multi-result function: {ger:?}"
    );
}

#[test]
fn ger_skips_unsupported_return_types() {
    let source = r#"package main

type User struct { name string }

func makeUser() User {
    x := 1
    _ = x
    return User{name: "a"}
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.is_empty(),
        "GER should skip unsupported Go return types: {ger:?}"
    );
}

#[test]
fn ger_skips_partially_unsupported_multi_returns() {
    let source = r#"package main

type User struct { name string }

func mixed() (int, User) {
    if true {
        ping()
    }
    return 1, User{name: "a"}
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.is_empty(),
        "GER should skip when any return component is unsupported: {ger:?}"
    );
}

#[test]
fn ger_named_return_values() {
    let source = r#"package main

func compute() (result int) {
    ping()
    result = 42
    return
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping()" && m.new_text == "return 0"),
        "GER should work with named return values: {ger:?}"
    );
}

#[test]
fn ger_method_receiver() {
    let source = r#"package main

type Server struct{}

func (s *Server) Handle() error {
    ping()
    return nil
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping()" && m.new_text == "return nil"),
        "GER should work on method receivers: {ger:?}"
    );
}

#[test]
fn ger_error_only_return() {
    let source = r#"package main

func validate() error {
    ping()
    return nil
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping()" && m.new_text == "return nil"),
        "GER should handle error-only return: {ger:?}"
    );
}

#[test]
fn ger_nested_in_if_block() {
    let source = r#"package main

func f() int {
    if true {
        ping()
    }
    return 0
}
"#;
    let ger = ger_mutants(source);
    assert!(
        !ger.is_empty(),
        "GER should fire on statements inside if blocks: {ger:?}"
    );
}

#[test]
fn ger_picks_correct_return_type_across_functions() {
    let source = r#"package main

func boolFn() bool {
    ping()
    return true
}

func intFn() int {
    ping()
    return 42
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping()" && m.new_text == "return false"),
        "boolFn should become return false: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping()" && m.new_text == "return 0"),
        "intFn should become return 0: {ger:?}"
    );
}
