use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "DAS")
        .collect()
}

#[test]
fn test_das_delete_to_delete_array() {
    let source = r#"
void f(int* p) {
    delete p;
}
"#;
    let das = slug_mutants(source);
    assert_eq!(das.len(), 1, "Should generate 1 DAS mutation: {das:?}");
    assert_eq!(das[0].old_text, "delete p");
    assert_eq!(das[0].new_text, "delete[] p");
}

#[test]
fn test_das_delete_array_to_delete() {
    let source = r#"
void f(int* arr) {
    delete[] arr;
}
"#;
    let das = slug_mutants(source);
    assert_eq!(das.len(), 1, "Should generate 1 DAS mutation: {das:?}");
    assert_eq!(das[0].old_text, "delete[] arr");
    assert_eq!(das[0].new_text, "delete arr");
}

#[test]
fn test_das_both_forms() {
    let source = r#"
void f(int* p, int* arr) {
    delete p;
    delete[] arr;
}
"#;
    let das = slug_mutants(source);
    assert_eq!(
        das.len(),
        2,
        "Should generate DAS for both delete forms: {das:?}"
    );
}

#[test]
fn test_das_in_comment_ignored() {
    let source = r#"
// delete p;
/* delete[] arr; */
int main() { return 0; }
"#;
    let das = slug_mutants(source);
    assert!(
        das.is_empty(),
        "DAS should not mutate inside comments: {das:?}"
    );
}

#[test]
fn test_das_does_not_touch_new() {
    let source = r#"
void f() {
    int* p = new int(42);
    int* arr = new int[10];
}
"#;
    let das = slug_mutants(source);
    assert!(
        das.is_empty(),
        "DAS should only target delete expressions, not new: {das:?}"
    );
}

#[test]
fn test_das_complex_operand() {
    let source = r#"
struct Obj { int* ptr; };
void f(Obj* o) {
    delete o->ptr;
}
"#;
    let das = slug_mutants(source);
    assert_eq!(
        das.len(),
        1,
        "DAS should handle complex delete operands: {das:?}"
    );
    assert!(
        das[0].new_text.contains("delete[]"),
        "Should swap to delete[]: {:?}",
        das[0].new_text
    );
}
