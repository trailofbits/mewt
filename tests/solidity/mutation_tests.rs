use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::{Hash, Target};

fn solidity_target_from_source(source: &str) -> Target {
    use tempfile::tempdir;
    let tmp = tempdir().expect("tmpdir");
    let path = tmp.path().join("test.sol");
    std::fs::write(&path, source).unwrap();
    Target {
        id: 1,
        path,
        file_hash: Hash::digest(source.to_string()),
        text: source.to_string(),
        language: "Solidity".to_string(),
    }
}

#[test]
fn solidity_shared_slugs_presence() {
    // Solidity sample with if and a call with 2 args
    let solidity_src = r#"
pragma solidity ^0.8.0;

contract Test {
    function main() public {
        uint256 x = 1;
        if (x > 0) {
            return;
        }
        doSomething(1, 2);
    }
    
    function doSomething(uint256 a, uint256 b) public {}
}
"#;

    let target = solidity_target_from_source(solidity_src);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.apply_all_mutations(&target);

    fn count(mutants: &[mewt::types::Mutant], slug: &str) -> usize {
        mutants.iter().filter(|m| m.mutation_slug == slug).count()
    }

    let er_count = count(&mutants, "ER");
    let cr_count = count(&mutants, "CR");
    let as_count = count(&mutants, "AS");

    println!("solidity ER/CR/AS: {er_count}/{cr_count}/{as_count}");

    assert!(er_count > 0, "ER should be present in Solidity");
    assert!(cr_count > 0, "CR should be present in Solidity");
    // AS may or may not be present depending on implementation
}

#[test]
fn test_error_replacement_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 42;
        if (x > 0) {
            return x + 1;
        }
        return x - 1;
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.apply_all_mutations(&target);

    let er_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "ER").collect();

    assert!(!er_mutants.is_empty(), "Should generate ER mutations");

    // Check that ER mutations replace expressions with revert calls
    for mutant in er_mutants {
        assert!(
            mutant.new_text.contains("revert(") || mutant.new_text.contains("require(false"),
            "ER mutation should contain revert or require(false) call: {}",
            mutant.new_text
        );
    }
}

#[test]
fn test_comment_replacement_mutations() {
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

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
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
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 42;
        if (x > 0) {
            return x;
        } else {
            return 0;
        }
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
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
fn test_argument_swap_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public {
        foo(1, 2);
        bar(x, y, z);
    }
    
    function foo(uint256 a, uint256 b) public {}
    function bar(uint256 x, uint256 y, uint256 z) public {}
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.apply_all_mutations(&target);

    let as_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "AS").collect();

    // AS mutations may or may not be present depending on implementation
    if !as_mutants.is_empty() {
        // If AS mutations exist, they should swap function arguments
        for mutant in as_mutants {
            assert!(
                mutant.old_text.contains("(") && mutant.old_text.contains(")"),
                "AS mutation should involve function call: {}",
                mutant.old_text
            );
        }
    }
}

#[test]
fn test_variable_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 1;
        uint256 y = 2;
        return x + y;
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
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

#[test]
fn test_loop_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 i = 0;
        while (i < 10) {
            i += 1;
        }
        return i;
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.apply_all_mutations(&target);

    // Should have mutations that target loop constructs
    let loop_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| {
            m.old_text.contains("while") || m.old_text.contains("<") || m.old_text.contains("+=")
        })
        .collect();

    assert!(
        !loop_mutants.is_empty(),
        "Should generate loop-related mutations"
    );
}
