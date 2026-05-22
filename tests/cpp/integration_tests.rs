use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::core::resolver::LanguageResolver;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::languages::cpp::resolver::CppLanguageResolver;
use mewt::types::{Mutant, Target};
use std::collections::HashSet;

pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_extension("C++", "cpp", content).into_parts()
}

pub(crate) fn mutants_for_slug(source: &str, slug: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    utils::mutants_for_slug(&engine, &target, slug)
}

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    utils::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}

#[test]
fn cpp_common_conformance_checks() {
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"
int add(int a, int b) {
    return a + b;
}
"#,
        comment_source: r#"
int add(int a, int b) {
    // This is a comment
    if (a > b) {
        return a;
    }
    return b;
}
"#,
        complex_source: r#"
#include <vector>

class Counter {
public:
    Counter() : value_(0) {}

    int increment() {
        value_++;
        return value_;
    }

    int process(const std::vector<int>& data) {
        if (data.empty()) {
            return 0;
        }

        int sum = 0;
        for (const auto& v : data) {
            sum += v;
        }
        return sum;
    }

private:
    int value_;
};
"#,
        line_coverage_source: r#"
int compute(int x) {
    int y = x + 1;
    if (x > 0) {
        return x;
    }
    return y;
}
"#,
    };

    let expectations = conformance::CommonConformanceExpectations {
        language_name: "C++",
        min_complex_mutants: 6,
    };

    conformance::run_common_language_checks(
        create_test_target,
        || Box::new(CppLanguageEngine::new()),
        sources,
        expectations,
    );
}

#[test]
fn test_basic_mutations() {
    let source = r#"
int add(int a, int b) {
    return a + b;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Should generate mutations for C++ code"
    );

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(slugs.contains("ER"), "Should generate ER mutations");
    assert!(slugs.contains("CR"), "Should generate CR mutations");
    assert!(slugs.contains("AOS"), "Should generate AOS mutations");
}

#[test]
fn test_do_while_condition_mutation() {
    let source = r#"
int sum_to(int n) {
    int total = 0;
    int i = 1;
    do {
        total += i;
        i++;
    } while (i <= n);
    return total;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let wf: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "WF").collect();
    assert_eq!(
        wf.len(),
        1,
        "Should generate WF mutation for do-while condition: {wf:?}"
    );
    assert!(
        wf[0].new_text.contains("false"),
        "WF should replace do-while condition with false: {}",
        wf[0].new_text
    );
}

#[test]
fn test_range_for_loop() {
    let source = r#"
int sum_vec(int* arr, int n) {
    int total = 0;
    for (int i = 0; i < n; i++) {
        total += arr[i];
    }
    return total;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // for_statement should get ER and CR
    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(slugs.contains("ER"), "for loop should get ER mutations");
    assert!(slugs.contains("CR"), "for loop should get CR mutations");
}

#[test]
fn test_range_based_for_gets_er_cr() {
    let source = r#"
#include <vector>
int sum(const std::vector<int>& vec) {
    int total = 0;
    for (const auto& v : vec) {
        total += v;
    }
    return total;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Range-based for should get ER and CR (the whole loop can be replaced/commented)
    let range_for_er = mutants
        .iter()
        .any(|m| m.mutation_slug == "ER" && m.old_text.contains("for"));
    let range_for_cr = mutants
        .iter()
        .any(|m| m.mutation_slug == "CR" && m.old_text.contains("for"));

    assert!(range_for_er, "Range-based for should get ER mutations");
    assert!(range_for_cr, "Range-based for should get CR mutations");
}

#[test]
fn test_example_file() {
    let source = conformance::read_example_source("tests/cpp/example.cpp");
    let (_dir, target) = create_test_target(&source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        mutants.len() > 20,
        "Example file should generate many mutations, got {}",
        mutants.len()
    );

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    let expected = ["ER", "CR", "IF", "IT", "AOS", "COS", "BL", "NR"];
    for slug in expected {
        assert!(
            slugs.contains(slug),
            "Example file should produce {} mutations",
            slug
        );
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
    let (_dir, target) = create_test_target(source);
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
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
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
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
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
    // Verify resolver handles supported extensions
    let resolver = CppLanguageResolver::new();
    for ext in ["cpp", "cc", "cxx", "hpp", "hxx"] {
        let resolved = resolver
            .resolve_for_extension(ext, None)
            .expect("extension should be recognized")
            .expect("resolution should succeed");
        assert_eq!(resolved, "C++");
    }
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
    let (_dir, target) = create_test_target(source);
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
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
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
    let (_dir, target) = create_test_target(source);
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
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(
        mutants.is_empty(),
        "Empty function body should produce zero mutations: {mutants:?}"
    );
}

#[test]
fn test_multiple_slugs_on_same_statement() {
    let source = r#"
bool f(int a, int b) {
    return a > b;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // The return statement should get ER, CR, and the expression should get COS
    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
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
    let (_dir, target) = create_test_target(source);
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
