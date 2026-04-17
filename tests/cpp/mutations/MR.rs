use crate::cpp::integration_tests::mutants_for_slug;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    mutants_for_slug(source, "MR")
}

#[test]
fn test_mr_std_move() {
    let source = r#"
void f(std::string s) {
    auto x = std::move(s);
}
"#;
    let mr = slug_mutants(source);
    assert_eq!(mr.len(), 1, "Should generate 1 MR mutation: {mr:?}");
    assert_eq!(mr[0].old_text, "std::move(s)");
    assert_eq!(mr[0].new_text, "s");
}

#[test]
fn test_mr_unqualified_move() {
    let source = r#"
using std::move;
void f(std::string s) {
    auto x = move(s);
}
"#;
    let mr = slug_mutants(source);
    assert_eq!(
        mr.len(),
        1,
        "Should generate MR for unqualified move(): {mr:?}"
    );
    assert_eq!(mr[0].old_text, "move(s)");
    assert_eq!(mr[0].new_text, "s");
}

#[test]
fn test_mr_ignores_other_functions() {
    let source = r#"
int compute(int x) { return x; }
void f() {
    auto y = compute(42);
}
"#;
    let mr = slug_mutants(source);
    assert!(
        mr.is_empty(),
        "MR should only target std::move/move, not other functions: {mr:?}"
    );
}

#[test]
fn test_mr_in_comment_ignored() {
    let source = r#"
// auto x = std::move(s);
/* move(s) */
int main() { return 0; }
"#;
    let mr = slug_mutants(source);
    assert!(
        mr.is_empty(),
        "MR should not mutate inside comments: {mr:?}"
    );
}

#[test]
fn test_mr_ignores_std_forward() {
    let source = r#"
template<typename T>
void f(T&& arg) {
    g(std::forward<T>(arg));
}
"#;
    let mr = slug_mutants(source);
    assert!(mr.is_empty(), "MR should not target std::forward: {mr:?}");
}

#[test]
fn test_mr_nested_in_function_call() {
    let source = r#"
void consume(std::string s) {}
void f(std::string s) {
    consume(std::move(s));
}
"#;
    let mr = slug_mutants(source);
    assert_eq!(
        mr.len(),
        1,
        "Should generate MR for std::move nested inside another call: {mr:?}"
    );
    assert_eq!(mr[0].old_text, "std::move(s)");
    assert_eq!(mr[0].new_text, "s");
}

#[test]
fn test_mr_ignores_multi_arg_move() {
    // std::move with iterator range (2 args) should not be touched
    let source = r#"
#include <algorithm>
void f(int* src, int* dst) {
    std::move(src, src + 10, dst);
}
"#;
    let mr = slug_mutants(source);
    assert!(
        mr.is_empty(),
        "MR should not target std::move with multiple arguments (iterator form): {mr:?}"
    );
}
