use std::path::PathBuf;

use mewt::types::{Hash, Mutant, Target};
use mewt::utils::calculate_line_offset;

fn strip_ansi(input: &str) -> String {
    // Basic ANSI escape removal
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            // Skip until 'm' or end
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[test]
fn test_display_single_line_replacement() {
    let source = "let x = 1 + 2;\nlet y = x;\n".to_string();
    // Use generic path and language - display should work regardless of language
    let path = PathBuf::from("/tmp/test.txt");
    let target = Target {
        id: 1,
        path,
        file_hash: Hash::digest(source.clone()),
        text: source.clone(),
        language: "Rust".to_string(), // Could be any language - display is language-agnostic
    };

    let old = "1 + 2".to_string();
    let new = "panic!()".to_string();
    let byte_offset = source.find(&old).unwrap() as u32;
    let line_offset = calculate_line_offset(&source, byte_offset as usize);

    let mutant = Mutant {
        id: 42,
        target_id: 0, // allow applying to any target
        byte_offset,
        line_offset,
        old_text: old.clone(),
        new_text: new.clone(),
        mutation_slug: "test-replace".to_string(),
    };

    let output = strip_ansi(&mutant.display(&target));
    assert!(
        output.contains("->"),
        "display should show an arrow: {output}"
    );
    assert!(output.contains("Line 1") || output.contains("Lines 1-1"));
    assert!(
        output.contains("panic!()"),
        "new_text should appear in display: {output}"
    );
    assert!(
        !output.contains("'' -> ''"),
        "should not show empty diff: {output}"
    );
}

#[test]
fn test_display_multi_line_replacement() {
    let source = "fn main() {\n    let a = 1;\n    let b = 2;\n}\n".to_string();
    let path = PathBuf::from("/tmp/test2.txt");
    let target = Target {
        id: 2,
        path,
        file_hash: Hash::digest(source.clone()),
        text: source.clone(),
        language: "Rust".to_string(), // Could be any language
    };

    let old = "1;\n    let b = 2;".to_string();
    let new = "panic!();".to_string();
    let byte_offset = source.find("1;\n    let b = 2;").unwrap() as u32;
    let line_offset = calculate_line_offset(&source, byte_offset as usize);

    let mutant = Mutant {
        id: 7,
        target_id: 0,
        byte_offset,
        line_offset,
        old_text: old.clone(),
        new_text: new.clone(),
        mutation_slug: "test-multiline".to_string(),
    };

    let output = strip_ansi(&mutant.display(&target));
    assert!(
        output.contains("Lines 2-3"),
        "should show multi-line range: {output}"
    );
    assert!(
        output.contains("panic!()"),
        "new_text should appear in display: {output}"
    );
}

#[test]
fn test_display_when_line_offset_zero() {
    // Mutation at very start of file
    let source = "abc\nxyz\n".to_string();
    let path = PathBuf::from("/tmp/test3.txt");
    let target = Target {
        id: 3,
        path,
        file_hash: Hash::digest(source.clone()),
        text: source.clone(),
        language: "Rust".to_string(), // Could be any language
    };

    let old = "abc".to_string();
    let new = "panic!()".to_string();
    let byte_offset = 0u32;
    let line_offset = 0u32; // first line

    let mutant = Mutant {
        id: 3,
        target_id: 0,
        byte_offset,
        line_offset,
        old_text: old,
        new_text: new.clone(),
        mutation_slug: "test-start".to_string(),
    };

    let output = strip_ansi(&mutant.display(&target));
    assert!(output.contains("Line 1"));
    assert!(output.contains(&new));
    assert!(!output.contains("'' -> ''"));
}
