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
