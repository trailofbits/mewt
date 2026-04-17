use crate::cpp::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::types::Mutant;

fn slug_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    CppLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "ER")
        .collect()
}

#[test]
fn test_error_replacement() {
    let source = r#"
int f() {
    int x = 42;
    return x;
}
"#;
    let er = slug_mutants(source);
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
    let (_tmp, target) = create_test_target(source);
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
fn test_existing_throw_not_replaced_by_er() {
    let source = r#"
void f(int x) {
    if (x < 0) {
        throw std::runtime_error("negative");
    }
}
"#;
    let (_tmp, target) = create_test_target(source);
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
fn test_auto_declaration_gets_er_cr() {
    let source = r#"
void f() {
    auto x = 42;
}
"#;
    let (_tmp, target) = create_test_target(source);
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
