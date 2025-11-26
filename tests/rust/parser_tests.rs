use mewt::LanguageRegistry;
use mewt::languages::rust::engine::RustLanguageEngine;
use std::fs;
use tempfile::tempdir;

fn create_temp_rust_file(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, content).expect("Failed to write test file");
    (temp_dir, file_path)
}

#[test]
fn test_parse_simple_function() {
    let source = r#"
fn main() {
    println!("Hello, world!");
}
"#;
    let (_temp_dir, file_path) = create_temp_rust_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(RustLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Rust", &source_content)
        .expect("Failed to parse");
    let root = tree.root_node();

    assert!(!root.has_error(), "Parse tree should not have errors");
    assert!(root.child_count() > 0, "Root should have children");
}

#[test]
fn test_parse_complex_rust_code() {
    let source = r#"
use std::collections::HashMap;

struct Counter {
    value: i32,
}

impl Counter {
    fn new() -> Self {
        Counter { value: 0 }
    }
    
    fn increment(&mut self) -> i32 {
        self.value += 1;
        self.value
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut counter = Counter::new();
    let result = counter.increment();
    println!("Result: {}", result);
    Ok(())
}
"#;
    let (_temp_dir, file_path) = create_temp_rust_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(RustLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Rust", &source_content)
        .expect("Failed to parse");
    let root = tree.root_node();

    assert!(
        !root.has_error(),
        "Parse tree should not have errors for complex code"
    );
    assert!(root.child_count() > 0, "Root should have children");
}

#[test]
fn test_parse_with_comments() {
    let source = r#"
// This is a line comment
fn main() {
    /* This is a block comment */
    let x = 1;
    // Another comment
    println!("{}", x);
}
"#;
    let (_temp_dir, file_path) = create_temp_rust_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(RustLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Rust", &source_content)
        .expect("Failed to parse");
    let root = tree.root_node();

    assert!(
        !root.has_error(),
        "Parse tree should not have errors with comments"
    );
}

#[test]
fn test_parse_hello_world_example() {
    let path = std::path::Path::new("tests/rust/examples/hello-world.rs");

    if path.exists() {
        let mut registry = LanguageRegistry::new();
        registry.register(RustLanguageEngine::new());
        let source_content = fs::read_to_string(path).expect("Failed to read file");
        let tree = registry
            .parse("Rust", &source_content)
            .expect("Failed to parse example");
        let root = tree.root_node();

        assert!(
            !root.has_error(),
            "Parse tree should not have errors for example"
        );
        assert!(root.child_count() > 0, "Root should have children");
    }
}

#[test]
fn test_parse_with_syntax_error() {
    let source = r#"
fn main( {
    println!("Hello");
}
"#;
    let (_temp_dir, file_path) = create_temp_rust_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(RustLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Rust", &source_content)
        .expect("Failed to parse");
    let root = tree.root_node();

    // Even with syntax errors, tree-sitter should still produce a tree
    assert!(
        root.child_count() > 0,
        "Root should still have children even with errors"
    );
}
