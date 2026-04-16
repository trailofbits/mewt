use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::{Mutant, Target};
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;

/// Helper to create a temporary Solidity target for tests.
pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.sol");
    std::fs::write(&file_path, content).expect("Failed to write test file");
    let target = Target {
        id: 1,
        path: file_path,
        file_hash: mewt::types::Hash::digest(content.to_string()),
        text: content.to_string(),
        language: "Solidity".to_string(),
    };
    (temp_dir, target)
}

/// Collect all mutants for the given slug from a Solidity source string.
pub(crate) fn mutants_for_slug(source: &str, slug: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    engine
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == slug)
        .collect()
}

/// Assert that only the provided slug produces specific snippets of text.
pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let selected: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == slug).collect();
    assert!(!selected.is_empty(), "expected at least one {slug} mutant");

    let normalize = |text: &str| text.trim().replace('\r', "");

    let mut covered_tokens: HashSet<&str> = HashSet::new();
    let mut unexpected_mutants: Vec<String> = Vec::new();

    for mutant in &selected {
        let matches: Vec<&str> = expected_new_texts
            .iter()
            .copied()
            .filter(|needle| mutant.new_text.contains(needle))
            .collect();

        if matches.is_empty() {
            unexpected_mutants.push(normalize(&mutant.new_text));
        } else {
            for needle in matches {
                covered_tokens.insert(needle);
            }
        }
    }

    assert!(
        unexpected_mutants.is_empty(),
        "found {slug} mutants with unexpected replacements: {unexpected_mutants:?}"
    );

    for expected in expected_new_texts {
        assert!(
            covered_tokens.contains(expected),
            "missing expected {slug} mutant containing: {expected}"
        );
    }
}

#[test]
fn test_basic_solidity_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function demo(uint256 x) public pure returns (uint256) {
        if (x > 0) {
            return x;
        }
        return 0;
    }
}
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    assert!(!mutants.is_empty(), "Should generate mutations");

    let slugs: HashSet<_> = mutants
        .iter()
        .map(|m| m.mutation_slug.chars().take(2).collect::<String>())
        .collect();
    assert!(slugs.len() > 1, "Should generate diverse mutation types");
}

#[test]
fn test_solidity_mutations_skip_comment_only_lines() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function demo(uint256 x) public pure returns (uint256) {
        // This is a comment
        if (x > 0) {
            return x;
        }
        return 0;
    }
}
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let comment_mutations = mutants
        .iter()
        .filter(|m| m.old_text.trim_start().starts_with("//"))
        .count();

    assert_eq!(
        comment_mutations, 0,
        "Mutations should not target comment-only lines"
    );
}

#[test]
fn test_solidity_engine_handles_complex_contracts() {
    let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract ComplexToken is ERC20, Ownable {
    mapping(address => bool) public blacklisted;
    uint256 public maxTransferAmount;

    event BlacklistUpdated(address user, bool status);
    event MaxTransferAmountUpdated(uint256 amount);

    constructor(
        string memory name,
        string memory symbol,
        uint256 initialSupply,
        uint256 _maxTransferAmount
    ) ERC20(name, symbol) {
        _mint(msg.sender, initialSupply * 10**decimals());
        maxTransferAmount = _maxTransferAmount;
    }

    function transfer(address to, uint256 amount) public override returns (bool) {
        require(!blacklisted[msg.sender], "Sender is blacklisted");
        require(!blacklisted[to], "Recipient is blacklisted");
        require(amount <= maxTransferAmount, "Transfer amount exceeds maximum");

        return super.transfer(to, amount);
    }

    function updateBlacklist(address user, bool status) external onlyOwner {
        blacklisted[user] = status;
        emit BlacklistUpdated(user, status);
    }

    function updateMaxTransferAmount(uint256 _maxTransferAmount) external onlyOwner {
        maxTransferAmount = _maxTransferAmount;
        emit MaxTransferAmountUpdated(_maxTransferAmount);
    }
}
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    let result = std::panic::catch_unwind(|| engine.mutate(&target));

    assert!(
        result.is_ok(),
        "Solidity engine should handle complex contracts without panicking"
    );

    if let Ok(mutants) = result {
        assert!(
            mutants.len() > 10,
            "Complex contracts should yield many mutations"
        );
    }
}

#[test]
fn test_solidity_mutations_cover_multiple_lines() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function demo(uint256 x) public pure returns (uint256) {
        uint256 y = x + 1;
        if (x > 0) {
            return x;
        }
        return y;
    }
}
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let mut lines: HashMap<usize, Vec<String>> = HashMap::new();
    for mutant in &mutants {
        lines
            .entry(mutant.line_offset as usize)
            .or_default()
            .push(mutant.mutation_slug.clone());
    }

    assert!(
        lines.len() > 1,
        "Mutations should touch multiple lines for reasonable coverage"
    );
}
