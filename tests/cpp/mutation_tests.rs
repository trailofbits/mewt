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
