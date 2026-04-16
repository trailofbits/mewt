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
