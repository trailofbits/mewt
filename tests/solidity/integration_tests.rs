use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::Target;
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;

/// Helper to create test target
fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
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

#[test]
fn test_mutation_count_comparison() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 42;
        if (x > 0) {
            return x;
        }
        return 0;
    }
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    // Get AST mutations
    let ast_engine = SolidityLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    println!("AST mutations: {}", ast_mutants.len());

    // AST should generate reasonable number of mutations
    assert!(
        !ast_mutants.is_empty(),
        "AST should generate some mutations"
    );

    // Check mutation types
    let ast_slugs: HashSet<_> = ast_mutants
        .iter()
        .map(|m| m.mutation_slug.chars().take(2).collect::<String>())
        .collect();

    println!("AST mutation types: {ast_slugs:?}");

    // Should generate diverse mutation types
    assert!(
        ast_slugs.len() > 1,
        "AST should generate diverse mutation types"
    );
}

#[test]
fn test_mutation_quality_comparison() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        // This is a comment
        uint256 x = 42;
        if (x > 0) {
            return x;
        }
        return 0;
    }
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    // Get AST mutations
    let ast_engine = SolidityLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    // Check comment handling (checking old_text for comment patterns)
    let ast_comment_mutations = ast_mutants
        .iter()
        .filter(|m| m.old_text.trim().starts_with("//"))
        .count();

    println!("AST comment mutations: {ast_comment_mutations}");

    // AST should avoid mutating comment-only lines
    assert_eq!(
        ast_comment_mutations, 0,
        "AST should not mutate comment-only lines"
    );
}

#[test]
fn test_complex_code_handling() {
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

    let (_temp_dir, target) = create_test_target(source);

    // Test that AST system can handle complex Solidity code
    let ast_engine = SolidityLanguageEngine::new();
    let ast_result = std::panic::catch_unwind(|| ast_engine.apply_all_mutations(&target));

    assert!(
        ast_result.is_ok(),
        "AST system should handle complex code without panicking"
    );

    if let Ok(ast_mutants) = ast_result {
        println!("Complex code - AST mutations: {}", ast_mutants.len());

        // Should generate substantial mutations for complex code
        assert!(
            ast_mutants.len() > 10,
            "AST should generate substantial mutations for complex code"
        );
    }
}

#[test]
fn test_mutation_overlap_analysis() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 42;
        uint256 y = x + 1;
        if (x > 0) {
            return x;
        }
        return y;
    }
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    let ast_engine = SolidityLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    // Analyze which lines are affected by mutations
    let mut ast_lines: HashMap<usize, Vec<String>> = HashMap::new();

    for mutant in &ast_mutants {
        ast_lines
            .entry(mutant.line_offset as usize)
            .or_default()
            .push(mutant.mutation_slug.clone());
    }

    println!("AST mutations by line: {ast_lines:?}");

    // Should affect multiple lines for decent coverage
    assert!(
        ast_lines.len() > 1,
        "AST mutations should affect multiple lines"
    );
}
