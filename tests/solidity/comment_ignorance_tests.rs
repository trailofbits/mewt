use crate::common;
use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::Target;

fn solidity_target_from_source(source: &str) -> Target {
    common::target_fixture_for_extension("Solidity", "sol", source).into_target()
}

#[test]
fn solidity_mutations_ignore_comment_regions() {
    let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract TestContract {
    // if (true) { revert("test"); }
    /* let x = 1 + 2; */
    uint256 public value;
    
    function setValue(uint256 _value) public {
        // Some comment
        value = _value;
        /* Another comment */
        if (value > 0) {
            emit ValueSet(value);
        }
    }
    
    event ValueSet(uint256 value);
}
"#;

    // NOTE: Keep this list in sync with source above.
    // Lines are 0-based and refer to fully-commented lines only.
    let commented_lines: &[usize] = &[1, 5, 6, 10, 12];

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Ensure none of the mutants originate from commented content (line or block)
    for m in &mutants {
        let line = m.line_offset as usize;
        assert!(
            !commented_lines.contains(&line),
            "mutated on commented line: slug={} line={} mutant={}",
            m.mutation_slug,
            line,
            m.display(&target),
        );
    }

    // Ensure CR does not double-wrap block-commented content
    let cr_nested = mutants
        .iter()
        .any(|m| m.mutation_slug == "CR" && m.new_text.contains("/* /*"));
    assert!(!cr_nested, "CR should not double-wrap commented content");
}
