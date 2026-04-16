use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Target;
use std::collections::HashSet;
use tempfile::tempdir;

fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.cpp");
    std::fs::write(&file_path, content).expect("Failed to write test file");
    let target = Target {
        id: 1,
        path: file_path,
        file_hash: mewt::types::Hash::digest(content.to_string()),
        text: content.to_string(),
        language: "C++".to_string(),
    };
    (temp_dir, target)
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
fn test_conditional_mutations() {
    let source = r#"
int abs_val(int x) {
    if (x < 0) {
        return -x;
    }
    return x;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(slugs.contains("IF"), "Should generate IF mutations");
    assert!(slugs.contains("IT"), "Should generate IT mutations");
    assert!(slugs.contains("COS"), "Should generate COS mutations");
}

#[test]
fn test_loop_mutations() {
    let source = r#"
int sum(int n) {
    int total = 0;
    while (n > 0) {
        total += n;
        n--;
    }
    return total;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(slugs.contains("WF"), "Should generate WF mutations");
    assert!(slugs.contains("AAOS"), "Should generate AAOS mutations");
}

#[test]
fn test_boolean_and_logical_mutations() {
    let source = r#"
bool check(bool a, bool b) {
    if (a && b) {
        return true;
    }
    return false;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(slugs.contains("BL"), "Should generate BL mutations");
    assert!(slugs.contains("LOS"), "Should generate LOS mutations");
}

#[test]
fn test_bitwise_mutations() {
    let source = r#"
int bitops(int a, int b) {
    int x = a & b;
    int y = a | b;
    int z = a << 2;
    return x + y + z;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let slugs: HashSet<_> = mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    assert!(slugs.contains("BOS"), "Should generate BOS mutations");
    assert!(slugs.contains("SOS"), "Should generate SOS mutations");
}

#[test]
fn test_negation_removal() {
    let source = r#"
bool check(bool flag) {
    if (!flag) {
        return false;
    }
    return true;
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let nr: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "NR").collect();
    assert_eq!(nr.len(), 1, "Should generate exactly 1 NR mutation");
    assert_eq!(nr[0].old_text, "!flag");
    assert_eq!(nr[0].new_text, "flag");
}

#[test]
fn test_argument_swap() {
    let source = r#"
int add(int a, int b) { return a + b; }
int main() {
    return add(1, 2);
}
"#;
    let (_dir, target) = create_test_target(source);
    let engine = CppLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let as_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "AS").collect();
    assert!(
        !as_mutants.is_empty(),
        "Should generate AS mutations for function calls with multiple args"
    );
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
    let source = std::fs::read_to_string("tests/cpp/examples/hello-world.cpp")
        .expect("Failed to read example file");
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
