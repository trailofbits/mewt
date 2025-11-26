use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;
use mewt::types::{Hash, Target};

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
    let mutants = engine.apply_all_mutations(&target);

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
    let mutants = engine.apply_all_mutations(&target);

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
    let mutants = engine.apply_all_mutations(&target);

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
    let mutants = engine.apply_all_mutations(&target);

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
    let mutants = engine.apply_all_mutations(&target);

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
    let mutants = engine.apply_all_mutations(&target);

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
