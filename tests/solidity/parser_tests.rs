use mewt::LanguageRegistry;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use std::fs;
use tempfile::tempdir;

fn create_temp_solidity_file(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.sol");
    fs::write(&file_path, content).expect("Failed to write test file");
    (temp_dir, file_path)
}

#[test]
fn test_parse_simple_contract() {
    let source = r#"
pragma solidity ^0.8.0;

contract SimpleContract {
    function hello() public pure returns (string memory) {
        return "Hello, World!";
    }
}
"#;
    let (_temp_dir, file_path) = create_temp_solidity_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(SolidityLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Solidity", &source_content)
        .expect("Failed to parse");
    let root = tree.root_node();

    assert!(!root.has_error(), "Parse tree should not have errors");
    assert!(root.child_count() > 0, "Root should have children");
}

#[test]
fn test_parse_complex_contract() {
    let source = r#"
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract ComplexContract is ERC20 {
    mapping(address => bool) public authorized;
    uint256 private _totalSupply;
    
    event AuthorizedUpdated(address indexed user, bool status);
    
    modifier onlyAuthorized() {
        require(authorized[msg.sender], "Not authorized");
        _;
    }
    
    constructor(string memory name, string memory symbol) ERC20(name, symbol) {
        authorized[msg.sender] = true;
        _totalSupply = 1000000 * 10**decimals();
        _mint(msg.sender, _totalSupply);
    }
    
    function authorize(address user, bool status) external onlyAuthorized {
        authorized[user] = status;
        emit AuthorizedUpdated(user, status);
    }
    
    function transfer(address to, uint256 amount) public override returns (bool) {
        require(authorized[msg.sender] || authorized[to], "Transfer not authorized");
        return super.transfer(to, amount);
    }
}
"#;
    let (_temp_dir, file_path) = create_temp_solidity_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(SolidityLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Solidity", &source_content)
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
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract CommentedContract {
    // This is a line comment
    uint256 public value;
    
    /* This is a block comment */
    function setValue(uint256 _value) public {
        // Another comment
        value = _value;
        /* Multi-line
           block comment */
    }
}
"#;
    let (_temp_dir, file_path) = create_temp_solidity_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(SolidityLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Solidity", &source_content)
        .expect("Failed to parse");
    let root = tree.root_node();

    assert!(
        !root.has_error(),
        "Parse tree should not have errors with comments"
    );
}

#[test]
fn test_parse_hello_world_example() {
    let path = std::path::Path::new("tests/solidity/examples/hello-world.sol");

    if path.exists() {
        let mut registry = LanguageRegistry::new();
        registry.register(SolidityLanguageEngine::new());
        let source_content = fs::read_to_string(path).expect("Failed to read file");
        let tree = registry
            .parse("Solidity", &source_content)
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
pragma solidity ^0.8.0;

contract BrokenContract {
    function broken( public pure returns (string memory) {
        return "This has a syntax error";
    }
}
"#;
    let (_temp_dir, file_path) = create_temp_solidity_file(source);

    let mut registry = LanguageRegistry::new();
    registry.register(SolidityLanguageEngine::new());
    let source_content = fs::read_to_string(&file_path).expect("Failed to read file");
    let tree = registry
        .parse("Solidity", &source_content)
        .expect("Failed to parse");
    let root = tree.root_node();

    // Even with syntax errors, tree-sitter should still produce a tree
    assert!(
        root.child_count() > 0,
        "Root should still have children even with errors"
    );
}
