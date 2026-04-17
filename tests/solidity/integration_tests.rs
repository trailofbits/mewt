use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::{Mutant, Target};

/// Helper to create a temporary Solidity target for tests.
pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_extension("Solidity", "sol", content).into_parts()
}

/// Collect all mutants for the given slug from a Solidity source string.
pub(crate) fn mutants_for_slug(source: &str, slug: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    utils::mutants_for_slug(&engine, &target, slug)
}

/// Assert that only the provided slug produces specific snippets of text.
pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = SolidityLanguageEngine::new();
    utils::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}

#[test]
fn solidity_common_conformance_checks() {
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"
pragma solidity ^0.8.0;

contract Test {
    function demo(uint256 x) public pure returns (uint256) {
        if (x > 0) {
            return x;
        }
        return 0;
    }
}
"#,
        comment_source: r#"
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
"#,
        complex_source: r#"
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
"#,
        line_coverage_source: r#"
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
"#,
    };

    let expectations = conformance::CommonConformanceExpectations {
        language_name: "Solidity",
        min_complex_mutants: 10,
    };

    conformance::run_common_language_checks(
        create_test_target,
        || Box::new(SolidityLanguageEngine::new()),
        sources,
        expectations,
    );
}

fn solidity_target_from_source(source: &str) -> Target {
    utils::target_fixture_for_extension("Solidity", "sol", source).into_target()
}

#[test]
fn solidity_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/solidity/example.sol");
    let (_tmp, target) = create_test_target(&source);
    let mutants = SolidityLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Solidity example file should generate mutants"
    );
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
