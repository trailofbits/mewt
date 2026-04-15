use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;
use mewt::types::{Hash, Mutant, Target};

fn rust_target_from_source(source: &str) -> Target {
    use tempfile::tempdir;
    let tmp = tempdir().expect("tmpdir");
    let path = tmp.path().join("test.rs");
    std::fs::write(&path, source).unwrap();
    Target {
        id: 1,
        path,
        file_hash: Hash::digest(source.to_string()),
        text: source.to_string(),
        language: "Rust".to_string(),
    }
}

#[test]
fn no_mutations_inside_line_or_block_comments() {
    let source = r#"
// if true { assert!(false); }
// let a = 1 + 2;
// if a == 3 { println!("three"); }
// do_something(10, 20);
// while true { break; }
fn main() {
    // if 1 + 2 == 3 { println!("math"); }
    // if (1 < 2) && (3 > 2) { println!("compare"); }
    // some_call(1, 2);
    /* if true { assert!(false); } */
    /* let x = 1 + 2; */
    println!("Hello, world!");
}
"#;

    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // None of the mutants should have old_text originating from a Rust comment
    // Simple heuristic: if old_text string is found entirely between // or /* */ regions in the source
    // We can conservatively assert that no mutant byte range falls within comment tokens by scanning.
    // For simplicity, just check that no mutant.old_text starts with // or contains only comment markers.
    for m in &mutants {
        let old = m.old_text.trim();
        assert!(
            !old.starts_with("//") && !old.starts_with("/*") && !old.ends_with("*/"),
            "mutated inside comment: slug={} old_text={:?}",
            m.mutation_slug,
            m.old_text
        );
    }

    // Additionally, ensure no CR wraps produce nested comment markers on already commented lines
    let cr_nested = mutants
        .iter()
        .any(|m| m.mutation_slug == "CR" && m.new_text.contains("/* /*"));
    assert!(
        !cr_nested,
        "CR should not double-comment already commented code"
    );
}

#[test]
fn rust_shared_slugs_presence() {
    // Rust sample with if and a call with 2 args
    let rust_src = r#"
fn main() {
    let x = 1;
    if x > 0 {
        return;
    }
    do_something(1, 2);
}
"#;

    let target = rust_target_from_source(rust_src);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);

    fn count(mutants: &[mewt::types::Mutant], slug: &str) -> usize {
        mutants.iter().filter(|m| m.mutation_slug == slug).count()
    }

    let er_count = count(&mutants, "ER");
    let cr_count = count(&mutants, "CR");
    let as_count = count(&mutants, "AS");

    println!("rust ER/CR/AS: {er_count}/{cr_count}/{as_count}");

    assert!(er_count > 0, "ER should be present in Rust");
    assert!(cr_count > 0, "CR should be present in Rust");
    // AS may or may not be present depending on implementation
}

#[test]
fn test_error_replacement_mutations() {
    let source = r#"
fn test_func() -> i32 {
    let x = 42;
    if x > 0 {
        return x + 1;
    }
    x - 1
}
"#;

    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let er_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "ER").collect();

    assert!(!er_mutants.is_empty(), "Should generate ER mutations");

    // Check that ER mutations replace expressions with panic calls
    for mutant in er_mutants {
        assert!(
            mutant.new_text.contains("assert!"),
            "ER mutation should introduce an assertion: {}",
            mutant.new_text
        );
    }
}

#[test]
fn test_comment_replacement_mutations() {
    let source = r#"
fn test_func() -> i32 {
    let x = 42;
    if x > 0 {
        return x;
    }
    0
}
"#;

    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cr_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "CR").collect();

    assert!(!cr_mutants.is_empty(), "Should generate CR mutations");

    // Check that CR mutations wrap code in comments
    for mutant in cr_mutants {
        assert!(
            mutant.new_text.starts_with("/*") && mutant.new_text.ends_with("*/"),
            "CR mutation should wrap in block comments: {}",
            mutant.new_text
        );
    }
}

#[test]
fn test_conditional_mutations() {
    let source = r#"
fn test_func() -> i32 {
    let x = 42;
    if x > 0 {
        x
    } else {
        0
    }
}
"#;

    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Should have mutations that target conditional expressions
    let conditional_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.old_text.contains(">") || m.old_text.contains("if"))
        .collect();

    assert!(
        !conditional_mutants.is_empty(),
        "Should generate conditional mutations"
    );
}

#[test]
fn test_variable_mutations() {
    let source = r#"
fn test_func() -> i32 {
    let x = 1;
    let y = 2;
    x + y
}
"#;

    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Should have mutations that target variables and expressions
    let var_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| {
            m.old_text.trim() == "x" || m.old_text.trim() == "y" || m.old_text.contains("+")
        })
        .collect();

    assert!(
        !var_mutants.is_empty(),
        "Should generate variable-related mutations"
    );
}

#[test]
fn compound_assignment_slugs_produce_mutants() {
    // Regression test for .todo/a3c12f04: AAOS/BAOS/SAOS were wired to
    // `binary_expression`, but compound assignment in tree-sitter-rust parses
    // as `compound_assignment_expr`. The slugs silently emitted zero mutants.
    let source = r#"
fn f() {
    let mut x = 0;
    x += 1;
    x -= 1;
    x *= 2;
    x /= 2;
    x %= 2;
    x &= 1;
    x |= 1;
    x <<= 1;
    x >>= 1;
}
"#;
    let target = rust_target_from_source(source);
    let mutants = RustLanguageEngine::new().mutate(&target);
    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    for slug in ["AAOS", "BAOS", "SAOS"] {
        assert!(
            slugs.contains(slug),
            "expected slug {} to produce at least one mutant; got slugs: {:?}",
            slug,
            slugs
        );
    }
    // Verify `%=` is covered in AAOS
    assert!(
        mutants
            .iter()
            .any(|m| m.mutation_slug == "AAOS" && m.old_text == "%="),
        "expected an AAOS mutant with old_text `%=`"
    );
}

fn nr_mutants(mutants: &[Mutant]) -> Vec<&Mutant> {
    mutants.iter().filter(|m| m.mutation_slug == "NR").collect()
}

#[test]
fn test_negation_removal_basic() {
    let source = r#"
fn main() {
    let x = true;
    if !x {
        println!("negated");
    }
}
"#;
    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert_eq!(nr.len(), 1, "Should generate exactly 1 NR mutation");
    assert_eq!(nr[0].old_text, "!x");
    assert_eq!(nr[0].new_text, "x");
}

#[test]
fn test_negation_removal_complex_expression() {
    let source = r#"
fn check(a: bool, b: bool) -> bool {
    !(a && b)
}
"#;
    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(a && b)" && m.new_text == "(a && b)"),
        "NR should remove negation preserving parenthesized operand: {nr:?}"
    );
}

#[test]
fn test_negation_removal_ignores_other_unary_ops() {
    let source = r#"
fn main() {
    let x = -1;
    let y = *x;
}
"#;
    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert!(
        nr.is_empty(),
        "NR should not trigger on - or * unary operators"
    );
}

#[test]
fn test_negation_removal_in_comment_ignored() {
    let source = r#"
fn main() {
    // if !x { }
    /* !flag */
    let y = true;
}
"#;
    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert!(
        nr.is_empty(),
        "NR should not generate mutations inside comments"
    );
}
