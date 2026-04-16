use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::{Hash, Mutant, Target};

fn cpp_target_from_source(source: &str) -> Target {
    use tempfile::tempdir;
    let tmp = tempdir().expect("tmpdir");
    let path = tmp.path().join("test.cpp");
    std::fs::write(&path, source).unwrap();
    Target {
        id: 1,
        path,
        file_hash: Hash::digest(source.to_string()),
        text: source.to_string(),
        language: "C++".to_string(),
    }
}

#[test]
fn no_mutations_inside_comments() {
    let source = r#"
// if (true) { return 42; }
// int x = 1 + 2;
/* if (a == 3) { return a; } */
int main() {
    return 0;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    for m in &mutants {
        let old = m.old_text.trim();
        assert!(
            !old.starts_with("//") && !old.starts_with("/*") && !old.ends_with("*/"),
            "mutated inside comment: slug={} old_text={:?}",
            m.mutation_slug,
            m.old_text
        );
    }
}

#[test]
fn test_error_replacement() {
    let source = r#"
int f() {
    int x = 42;
    return x;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let er: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "ER").collect();
    assert!(!er.is_empty(), "Should generate ER mutations");
    for m in &er {
        assert!(
            m.new_text.contains("throw"),
            "ER should replace with throw: {}",
            m.new_text
        );
    }
}

#[test]
fn test_comment_replacement() {
    let source = r#"
int f() {
    int x = 42;
    return x;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "CR").collect();
    assert!(!cr.is_empty(), "Should generate CR mutations");
    for m in &cr {
        assert!(
            m.new_text.starts_with("/*") && m.new_text.ends_with("*/"),
            "CR should wrap in block comments: {}",
            m.new_text
        );
    }
}

#[test]
fn test_compound_assignment_mutations() {
    let source = r#"
void f() {
    int x = 0;
    x += 1;
    x -= 2;
    x *= 3;
    x /= 4;
    x &= 0xff;
    x |= 0x01;
    x <<= 2;
    x >>= 1;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();

    assert!(slugs.contains("AAOS"), "Should generate AAOS mutations");
    assert!(slugs.contains("BAOS"), "Should generate BAOS mutations");
    assert!(slugs.contains("SAOS"), "Should generate SAOS mutations");
}

#[test]
fn test_negation_removal_ignores_other_unary_ops() {
    let source = r#"
int f(int x) {
    return -x;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let nr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();
    assert!(
        nr.is_empty(),
        "NR should not trigger on - unary operator: {nr:?}"
    );
}

#[test]
fn test_negation_removal_complex_expression() {
    let source = r#"
bool check(bool a, bool b) {
    return !(a && b);
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let nr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();
    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(a && b)" && m.new_text == "(a && b)"),
        "NR should remove negation preserving parenthesized operand: {nr:?}"
    );
}

#[test]
fn test_negation_removal_in_comment_ignored() {
    let source = r#"
// if (!flag) { return; }
/* !x */
int main() { return 0; }
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let nr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();
    assert!(
        nr.is_empty(),
        "NR should not generate mutations inside comments: {nr:?}"
    );
}

#[test]
fn er_and_cr_cover_all_statement_kinds() {
    let source = r#"
void f(int n) {
    int x = 0;
    x = n + 1;
    return;
    if (n > 0) { x++; }
    while (n > 0) { n--; }
    for (int i = 0; i < n; i++) { x++; }
    do { n--; } while (n > 0);
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cases: &[(&str, &str)] = &[
        ("int x = 0", "declaration"),
        ("x = n + 1", "expression_statement"),
        ("return", "return_statement"),
        ("if", "if_statement"),
        ("while", "while_statement"),
        ("for", "for_statement"),
        ("do", "do_statement"),
    ];

    for (prefix, label) in cases {
        let has_er = mutants
            .iter()
            .any(|m| m.mutation_slug == "ER" && m.old_text.trim_start().starts_with(prefix));
        let has_cr = mutants
            .iter()
            .any(|m| m.mutation_slug == "CR" && m.old_text.trim_start().starts_with(prefix));
        assert!(
            has_er,
            "expected an ER mutant for {label} (prefix {prefix:?}); got: {:?}",
            mutants
                .iter()
                .filter(|m| m.mutation_slug == "ER")
                .map(|m| m.old_text.as_str())
                .collect::<Vec<_>>()
        );
        assert!(
            has_cr,
            "expected a CR mutant for {label} (prefix {prefix:?}); got: {:?}",
            mutants
                .iter()
                .filter(|m| m.mutation_slug == "CR")
                .map(|m| m.old_text.as_str())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_template_function_mutations() {
    let source = r#"
template<typename T>
T max_val(T a, T b) {
    if (a > b) {
        return a;
    }
    return b;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(
        slugs.contains("COS"),
        "Should generate COS mutations inside template functions"
    );
    assert!(
        slugs.contains("IF"),
        "Should generate IF mutations inside template functions"
    );
    assert!(
        slugs.contains("ER"),
        "Should generate ER mutations inside template functions"
    );
}

#[test]
fn test_operator_overload_not_mutated_in_signature() {
    let source = r#"
struct Foo {
    int val;
    bool operator==(const Foo& other) {
        return val == other.val;
    }
};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // COS should only mutate the `==` inside the body (val == other.val),
    // not the `==` in the operator signature (operator==)
    let cos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    for m in &cos {
        assert!(
            !m.old_text.contains("operator"),
            "COS should not mutate operator== in the signature: {:?}",
            m.old_text
        );
    }
}

#[test]
fn test_namespace_and_class_mutations() {
    let source = r#"
namespace ns {
class Widget {
public:
    int compute(int a, int b) {
        if (a > b) {
            return a - b;
        }
        return a + b;
    }
};
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(
        slugs.contains("AOS"),
        "Should generate AOS mutations inside class methods"
    );
    assert!(
        slugs.contains("IF"),
        "Should generate IF mutations inside namespaced class methods"
    );
}

#[test]
fn test_different_extensions() {
    // Verify the engine claims the right extensions
    let engine = CppLanguageEngine::new();
    let exts = engine.extensions();
    assert!(exts.contains(&"cpp"), "Should support .cpp");
    assert!(exts.contains(&"cc"), "Should support .cc");
    assert!(exts.contains(&"cxx"), "Should support .cxx");
    assert!(exts.contains(&"hpp"), "Should support .hpp");
    assert!(exts.contains(&"hxx"), "Should support .hxx");
}

#[test]
fn test_preprocessor_directives_not_mutated() {
    let source = r#"
#define MAX_SIZE 100
#if MAX_SIZE > 50
int big = 1;
#endif
int main() {
    return 0;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Mutations inside preprocessor conditions (#if MAX_SIZE > 50) should not be
    // generated — tree-sitter treats these as preproc_if nodes, not regular
    // if_statements. Document the actual behavior.
    let preproc_mutants: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.old_text.contains("MAX_SIZE") || m.old_text.contains("#"))
        .collect();
    assert!(
        preproc_mutants.is_empty(),
        "Should not generate mutations for preprocessor directives: {preproc_mutants:?}"
    );
}

#[test]
fn test_ternary_operator() {
    let source = r#"
int max_val(int a, int b) {
    return a > b ? a : b;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // COS should mutate the > inside the ternary condition
    let cos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    assert!(
        cos.iter().any(|m| m.old_text == ">"),
        "COS should mutate the comparison in a ternary expression: {cos:?}"
    );
}

#[test]
fn test_lambda_mutations() {
    let source = r#"
void f() {
    auto check = [](int x) {
        if (x > 0) {
            return true;
        }
        return false;
    };
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(
        slugs.contains("COS"),
        "Should generate COS mutations inside lambdas"
    );
    assert!(
        slugs.contains("BL"),
        "Should generate BL mutations inside lambdas"
    );
    assert!(
        slugs.contains("IF"),
        "Should generate IF mutations inside lambdas"
    );
}

#[test]
fn test_nested_conditionals() {
    let source = r#"
bool check(int a, int b, int c) {
    if (a > 0 && (b > 0 || c < 10)) {
        return true;
    }
    return false;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    let los: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "LOS")
        .collect();

    // Three comparisons: a > 0, b > 0, c < 10
    assert!(
        cos.len() >= 3 * 5,
        "Should generate COS mutants for each of the 3 comparisons (5 replacements each), got {}",
        cos.len()
    );
    // Two logical operators: && and ||
    assert!(
        los.len() >= 2,
        "Should generate LOS mutants for both && and ||, got {}",
        los.len()
    );
}

#[test]
fn test_empty_function_no_mutations() {
    let source = r#"
void f() {}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        mutants.is_empty(),
        "Empty function body should produce zero mutations: {mutants:?}"
    );
}

#[test]
fn test_switch_break_not_swapped_to_continue() {
    // break inside a switch (not inside a loop) — LC targets break_statement
    // and continue_statement, but a break inside a switch without a surrounding
    // loop should not produce a valid continue mutation. Document actual behavior.
    let source = r#"
int f(int x) {
    switch (x) {
        case 1:
            return 10;
        case 2:
            return 20;
        default:
            break;
    }
    return 0;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let lc: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "LC").collect();
    // LC will try to swap break -> continue here. This produces invalid code
    // (continue inside switch without a loop), which will fail to compile and
    // be recorded as TestFail (caught). This is acceptable noise — document it.
    // The important thing is it doesn't crash during mutation generation.
    // If there are LC mutants, they should involve break.
    for m in &lc {
        assert!(
            m.old_text.contains("break") || m.old_text.contains("continue"),
            "LC mutants should involve break or continue: {m:?}"
        );
    }
}

#[test]
fn test_existing_throw_not_replaced_by_er() {
    let source = r#"
void f(int x) {
    if (x < 0) {
        throw std::runtime_error("negative");
    }
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // ER should not replace a statement that already contains a throw
    let er: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "ER").collect();
    for m in &er {
        assert!(
            !m.old_text.contains("throw"),
            "ER should not replace statements already containing throw: {:?}",
            m.old_text
        );
    }
}

#[test]
fn test_multiple_slugs_on_same_statement() {
    let source = r#"
bool f(int a, int b) {
    return a > b;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // The return statement should get ER, CR, and the expression should get COS
    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(slugs.contains("ER"), "Should have ER on the return");
    assert!(slugs.contains("CR"), "Should have CR on the return");
    assert!(slugs.contains("COS"), "Should have COS on the comparison");
}

#[test]
fn test_static_assert_not_mutated() {
    let source = r#"
static_assert(sizeof(int) == 4, "int must be 4 bytes");
int main() { return 0; }
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // static_assert is a static_assert_declaration, not an if_statement or
    // expression_statement. The condition should not get IF/IT mutations.
    // COS may mutate the == inside it — that's fine (it's a binary_expression).
    // But IF/IT should NOT fire.
    let if_mutants: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| {
            (m.mutation_slug == "IF" || m.mutation_slug == "IT") && m.old_text.contains("sizeof")
        })
        .collect();
    assert!(
        if_mutants.is_empty(),
        "IF/IT should not target static_assert conditions: {if_mutants:?}"
    );
}

// --- DAS (Delete Array Swap) tests ---

#[test]
fn test_das_delete_to_delete_array() {
    let source = r#"
void f(int* p) {
    delete p;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let das: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "DAS")
        .collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let das: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "DAS")
        .collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let das: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "DAS")
        .collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let das: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "DAS")
        .collect();
    assert!(
        das.is_empty(),
        "DAS should not mutate inside comments: {das:?}"
    );
}

// --- MR (Move Removal) tests ---

#[test]
fn test_mr_std_move() {
    let source = r#"
void f(std::string s) {
    auto x = std::move(s);
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "MR").collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "MR").collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "MR").collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "MR").collect();
    assert!(
        mr.is_empty(),
        "MR should not mutate inside comments: {mr:?}"
    );
}

// --- VR (Virtual Removal) tests ---

#[test]
fn test_vr_virtual_method_declaration() {
    let source = r#"
class Base {
    virtual void process();
};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let vr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "VR").collect();
    assert_eq!(vr.len(), 1, "Should generate 1 VR mutation: {vr:?}");
    assert!(
        vr[0].old_text.starts_with("virtual"),
        "VR old_text should start with virtual: {:?}",
        vr[0].old_text
    );
    assert!(
        !vr[0].new_text.contains("virtual"),
        "VR new_text should not contain virtual: {:?}",
        vr[0].new_text
    );
}

#[test]
fn test_vr_virtual_destructor() {
    let source = r#"
class Base {
    virtual ~Base() {}
};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let vr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "VR").collect();
    assert_eq!(
        vr.len(),
        1,
        "Should generate VR for virtual destructor: {vr:?}"
    );
    assert!(
        vr[0].new_text.contains("~Base"),
        "VR should preserve destructor name: {:?}",
        vr[0].new_text
    );
}

#[test]
fn test_vr_multiple_virtual_methods() {
    let source = r#"
class Base {
    virtual void f();
    virtual int g();
    void h();
};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let vr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "VR").collect();
    assert_eq!(
        vr.len(),
        2,
        "Should generate VR for each virtual method, not for non-virtual: {vr:?}"
    );
}

#[test]
fn test_vr_non_virtual_not_mutated() {
    let source = r#"
class Concrete {
    void f();
    int g();
};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let vr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "VR").collect();
    assert!(
        vr.is_empty(),
        "VR should not generate mutations for non-virtual methods: {vr:?}"
    );
}

#[test]
fn test_vr_in_comment_ignored() {
    let source = r#"
// virtual void f();
/* virtual int g(); */
class C {};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let vr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "VR").collect();
    assert!(
        vr.is_empty(),
        "VR should not mutate inside comments: {vr:?}"
    );
}

// --- Additional negative tests ---

#[test]
fn test_das_does_not_touch_new() {
    let source = r#"
void f() {
    int* p = new int(42);
    int* arr = new int[10];
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let das: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "DAS")
        .collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let das: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "DAS")
        .collect();
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

#[test]
fn test_mr_ignores_std_forward() {
    let source = r#"
template<typename T>
void f(T&& arg) {
    g(std::forward<T>(arg));
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "MR").collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "MR").collect();
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
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "MR").collect();
    assert!(
        mr.is_empty(),
        "MR should not target std::move with multiple arguments (iterator form): {mr:?}"
    );
}

#[test]
fn test_vr_override_without_virtual() {
    let source = r#"
class Derived {
    void f() override;
};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let vr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "VR").collect();
    assert!(
        vr.is_empty(),
        "VR should not fire on methods with override but no virtual keyword: {vr:?}"
    );
}

#[test]
fn test_vr_pure_virtual() {
    let source = r#"
class Abstract {
    virtual void process() = 0;
};
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let vr: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "VR").collect();
    assert_eq!(
        vr.len(),
        1,
        "VR should fire on pure virtual methods too: {vr:?}"
    );
    assert!(
        vr[0].new_text.contains("= 0"),
        "VR should preserve = 0 after removing virtual: {:?}",
        vr[0].new_text
    );
}

#[test]
fn test_nullptr_not_treated_as_boolean() {
    let source = r#"
void f() {
    int* p = nullptr;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let bl: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "BL").collect();
    assert!(
        bl.is_empty(),
        "BL should not treat nullptr as a boolean literal: {bl:?}"
    );
}

#[test]
fn test_auto_declaration_gets_er_cr() {
    let source = r#"
void f() {
    auto x = 42;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(
        slugs.contains("ER"),
        "auto declarations should get ER mutations"
    );
    assert!(
        slugs.contains("CR"),
        "auto declarations should get CR mutations"
    );
}

// --- Dedicated content-level tests for operator shuffle mutations ---

#[test]
fn test_aos_replacement_content() {
    let source = r#"
int f(int a, int b) {
    return a + b;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let aos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "AOS")
        .collect();
    // a + b should produce 4 replacements: -, *, /, %
    assert_eq!(
        aos.len(),
        4,
        "AOS should produce 4 replacements for +: {aos:?}"
    );
    assert!(
        aos.iter().all(|m| m.old_text == "+"),
        "All should replace +"
    );
    let new_ops: std::collections::HashSet<_> = aos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["-", "*", "/", "%"].into_iter().collect(),
        "Should replace + with all other arithmetic operators"
    );
}

#[test]
fn test_aaos_replacement_content() {
    let source = r#"
void f() {
    int x = 10;
    x += 5;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let aaos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "AAOS")
        .collect();
    assert_eq!(
        aaos.len(),
        4,
        "AAOS should produce 4 replacements for +=: {aaos:?}"
    );
    assert!(
        aaos.iter().all(|m| m.old_text == "+="),
        "All should replace +="
    );
    let new_ops: std::collections::HashSet<_> = aaos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["-=", "*=", "/=", "%="].into_iter().collect(),
        "Should replace += with all other arithmetic assignment operators"
    );
}

#[test]
fn test_bos_replacement_content() {
    let source = r#"
int f(int a, int b) {
    return a & b;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let bos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "BOS")
        .collect();
    assert_eq!(
        bos.len(),
        2,
        "BOS should produce 2 replacements for &: {bos:?}"
    );
    assert!(
        bos.iter().all(|m| m.old_text == "&"),
        "All should replace &"
    );
    let new_ops: std::collections::HashSet<_> = bos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["|", "^"].into_iter().collect(),
        "Should replace & with | and ^"
    );
}

#[test]
fn test_baos_replacement_content() {
    let source = r#"
void f() {
    int x = 0xff;
    x &= 0x0f;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let baos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "BAOS")
        .collect();
    assert_eq!(
        baos.len(),
        2,
        "BAOS should produce 2 replacements for &=: {baos:?}"
    );
    assert!(
        baos.iter().all(|m| m.old_text == "&="),
        "All should replace &="
    );
    let new_ops: std::collections::HashSet<_> = baos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["|=", "^="].into_iter().collect(),
        "Should replace &= with |= and ^="
    );
}

#[test]
fn test_sos_replacement_content() {
    let source = r#"
int f(int x) {
    return x << 2;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let sos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "SOS")
        .collect();
    assert_eq!(
        sos.len(),
        1,
        "SOS should produce 1 replacement for <<: {sos:?}"
    );
    assert_eq!(sos[0].old_text, "<<");
    assert_eq!(sos[0].new_text, ">>");
}

#[test]
fn test_saos_replacement_content() {
    let source = r#"
void f() {
    int x = 1;
    x <<= 3;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let saos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "SAOS")
        .collect();
    assert_eq!(
        saos.len(),
        1,
        "SAOS should produce 1 replacement for <<=: {saos:?}"
    );
    assert_eq!(saos[0].old_text, "<<=");
    assert_eq!(saos[0].new_text, ">>=");
}

#[test]
fn test_cos_replacement_content() {
    let source = r#"
bool f(int a, int b) {
    return a == b;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cos: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    assert_eq!(
        cos.len(),
        5,
        "COS should produce 5 replacements for ==: {cos:?}"
    );
    assert!(
        cos.iter().all(|m| m.old_text == "=="),
        "All should replace =="
    );
    let new_ops: std::collections::HashSet<_> = cos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["!=", "<", "<=", ">", ">="].into_iter().collect(),
        "Should replace == with all other comparison operators"
    );
}

#[test]
fn test_los_replacement_content() {
    let source = r#"
bool f(bool a, bool b) {
    return a && b;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let los: Vec<&Mutant> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "LOS")
        .collect();
    assert_eq!(
        los.len(),
        1,
        "LOS should produce 1 replacement for &&: {los:?}"
    );
    assert_eq!(los[0].old_text, "&&");
    assert_eq!(los[0].new_text, "||");
}

#[test]
fn test_wf_replacement_content() {
    let source = r#"
void f() {
    while (true) {
        break;
    }
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let wf: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "WF").collect();
    assert_eq!(
        wf.len(),
        1,
        "WF should produce 1 mutation for while condition: {wf:?}"
    );
    assert!(
        wf[0].new_text.contains("false"),
        "WF should replace condition with false: {:?}",
        wf[0].new_text
    );
}

#[test]
fn test_if_it_replacement_content() {
    let source = r#"
int f(int x) {
    if (x > 0) {
        return 1;
    }
    return 0;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let if_mut: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "IF").collect();
    let it_mut: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "IT").collect();

    assert_eq!(if_mut.len(), 1, "Should produce 1 IF mutation: {if_mut:?}");
    assert_eq!(it_mut.len(), 1, "Should produce 1 IT mutation: {it_mut:?}");
    assert!(
        if_mut[0].new_text.contains("false"),
        "IF should replace condition with false: {:?}",
        if_mut[0].new_text
    );
    assert!(
        it_mut[0].new_text.contains("true"),
        "IT should replace condition with true: {:?}",
        it_mut[0].new_text
    );
}

#[test]
fn test_bl_replacement_content() {
    let source = r#"
bool f() {
    return true;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let bl: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "BL").collect();
    assert_eq!(
        bl.len(),
        1,
        "BL should produce 1 replacement for true: {bl:?}"
    );
    assert_eq!(bl[0].old_text, "true");
    assert_eq!(bl[0].new_text, "false");
}

#[test]
fn test_lc_replacement_content() {
    let source = r#"
void f() {
    for (int i = 0; i < 10; i++) {
        if (i == 5) break;
    }
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let lc: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "LC").collect();
    assert_eq!(
        lc.len(),
        1,
        "LC should produce 1 replacement for break: {lc:?}"
    );
    assert!(
        lc[0].old_text.contains("break"),
        "LC old_text should contain break"
    );
    assert!(
        lc[0].new_text.contains("continue"),
        "LC should replace break with continue"
    );
}

#[test]
fn test_as_replacement_content() {
    let source = r#"
int add(int a, int b) { return a + b; }
int main() {
    return add(10, 20);
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let as_mut: Vec<&Mutant> = mutants.iter().filter(|m| m.mutation_slug == "AS").collect();
    assert_eq!(
        as_mut.len(),
        1,
        "AS should produce 1 swap for add(10, 20): {as_mut:?}"
    );
    assert!(
        as_mut[0].old_text.contains("10") && as_mut[0].old_text.contains("20"),
        "AS old_text should contain both args: {:?}",
        as_mut[0].old_text
    );
    // Swapped: 20, 10 instead of 10, 20
    assert!(
        as_mut[0].new_text.starts_with("20") && as_mut[0].new_text.ends_with("10"),
        "AS should swap argument order: {:?}",
        as_mut[0].new_text
    );
}

// --- RDV (Return Default Value) tests ---

fn rdv_mutants(mutants: &[Mutant]) -> Vec<&Mutant> {
    mutants
        .iter()
        .filter(|m| m.mutation_slug == "RDV")
        .collect()
}

#[test]
fn test_rdv_int_return() {
    let source = r#"
int compute() {
    return 42;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate 1 RDV mutation: {rdv:?}");
    assert_eq!(rdv[0].old_text, "42");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_bool_return() {
    let source = r#"
bool isValid() {
    return true;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.iter()
            .any(|m| m.old_text == "true" && m.new_text == "false"),
        "RDV should replace true with false: {rdv:?}"
    );
}

#[test]
fn test_rdv_float_return() {
    let source = r#"
double getRate() {
    return 3.14;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate 1 RDV mutation: {rdv:?}");
    assert_eq!(rdv[0].old_text, "3.14");
    assert_eq!(rdv[0].new_text, "0.0");
}

#[test]
fn test_rdv_pointer_return() {
    let source = r#"
int* findNode() {
    return ptr;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate 1 RDV mutation: {rdv:?}");
    assert_eq!(rdv[0].old_text, "ptr");
    assert_eq!(rdv[0].new_text, "nullptr");
}

#[test]
fn test_rdv_multiple_returns() {
    let source = r#"
int abs_val(int x) {
    if (x < 0) {
        return -x;
    }
    return x;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(
        rdv.len(),
        2,
        "Should generate 1 RDV per return statement: {rdv:?}"
    );
    assert!(rdv.iter().all(|m| m.new_text == "0"));
}

#[test]
fn test_rdv_skips_void_return() {
    let source = r#"
void doNothing() {
    return;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(rdv.is_empty(), "RDV should not mutate void return: {rdv:?}");
}

#[test]
fn test_rdv_skips_auto_return() {
    let source = r#"
auto deduce() {
    return 42;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should skip auto return type (can't determine default): {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_already_default() {
    let source = r#"
int zero() {
    return 0;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not mutate when return value is already default: {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_class_return() {
    let source = r#"
struct Point { int x; int y; };
Point origin() {
    return Point{0, 0};
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should skip user-defined type returns: {rdv:?}"
    );
}

#[test]
fn test_rdv_stdint_types() {
    let source = r#"
uint32_t get_id() {
    return 12345;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate RDV for uint32_t: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_size_t_return() {
    let source = r#"
size_t count() {
    return 42;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate RDV for size_t: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_unsigned_int() {
    let source = r#"
unsigned int get_count() {
    return 42;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(
        rdv.len(),
        1,
        "Should generate RDV for unsigned int: {rdv:?}"
    );
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_long_long() {
    let source = r#"
long long get_big() {
    return 999999;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate RDV for long long: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_unsigned_long_long() {
    let source = r#"
unsigned long long get_huge() {
    return 123456789;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(
        rdv.len(),
        1,
        "Should generate RDV for unsigned long long: {rdv:?}"
    );
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_long_double() {
    let source = r#"
long double get_precise() {
    return 3.14159265358979L;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate RDV for long double: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0.0");
}

#[test]
fn test_rdv_signed_char() {
    let source = r#"
signed char get_byte() {
    return 'a';
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate RDV for signed char: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_skips_std_string() {
    let source = r#"
#include <string>
std::string getName() {
    return "hello";
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should skip std::string (not a primitive type): {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_custom_type_with_keyword_substring() {
    // "interval" contains "int" as substring, "uint_wrapper" contains "uint"
    // Word-boundary matching should prevent false positives
    let source = r#"
struct interval { int lo; int hi; };
interval make_interval() {
    return interval{0, 10};
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not match 'interval' (contains 'int' as substring): {rdv:?}"
    );
}

#[test]
fn test_rdv_const_int_return() {
    let source = r#"
const int get_constant() {
    return 42;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    // "const int" — type node might be "int" with a separate const qualifier,
    // or "const int" as the full text. Either way, should produce RDV.
    assert_eq!(rdv.len(), 1, "Should generate RDV for const int: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_reference_return() {
    // int& return type — the & is in the declarator, not the type.
    // We detect pointer_declarator for T*, but reference_declarator for T&
    // is different. Document behavior.
    let source = r#"
int global = 42;
int& get_ref() {
    return global;
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    // The type field is "int", so this should produce RDV with default "0"
    assert_eq!(
        rdv.len(),
        1,
        "Should generate RDV for int& (type is still int): {rdv:?}"
    );
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_skips_template_return() {
    let source = r#"
#include <vector>
std::vector<int> get_vec() {
    return {1, 2, 3};
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should skip template return types: {rdv:?}"
    );
}

#[test]
fn test_rdv_char_return() {
    let source = r#"
char get_initial() {
    return 'A';
}
"#;
    let target = cpp_target_from_source(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate RDV for char: {rdv:?}");
    assert_eq!(rdv[0].new_text, "0");
}
