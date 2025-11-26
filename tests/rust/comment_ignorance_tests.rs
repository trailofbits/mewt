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
fn rust_mutations_ignore_comment_regions() {
    let source = r#"// if true { assert!(false); }
// let x = 1 + 2;
// if 1 < 2 { let y = 3; }
// foo(10, 20);
// while true { break; }
fn main() {
    
    let x: i32 = 1 + 2;
    if x > 0 { return; }
}
"#;

    // NOTE: Keep this list in sync with source above.
    // Lines are 0-based and refer to fully-commented lines only.
    let commented_lines: &[usize] = &[0, 1, 2, 3, 4];

    let target = rust_target_from_source(source);
    let engine = RustLanguageEngine::new();
    let mutants = engine.apply_all_mutations(&target);

    for m in &mutants {
        let line = m.line_offset as usize;
        assert!(
            !commented_lines.contains(&line),
            "mutated on commented line: slug={} line={}",
            m.mutation_slug,
            line
        );
    }

    // Ensure CR does not double-wrap block-commented content
    let cr_nested = mutants
        .iter()
        .any(|m| m.mutation_slug == "CR" && m.new_text.contains("/* /*"));
    assert!(!cr_nested, "CR should not double-wrap commented content");
}
