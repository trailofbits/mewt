use crate::solidity::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::Mutant;

fn ger_mutants(source: &str) -> Vec<Mutant> {
    let (_tmp_dir, target) = create_test_target(source);
    SolidityLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "GER")
        .collect()
}

#[test]
fn ger_supports_basic_solidity_returns() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function unitFn() public pure {
        ping();
    }

    function tupleFn() public pure returns (uint256, bool) {
        ping();
        return (x, true);
    }

    function scalarFn() public pure returns (uint256) {
        ping();
        return 1;
    }
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return;"),
        "expected return; for no-return function: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return 0;"),
        "expected return 0; for scalar return: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return (0, false);"),
        "expected return (0, false); for tuple return: {ger:?}"
    );
}

#[test]
fn ger_skips_unsupported_return_types() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    struct User { uint256 id; }

    function makeUser() public pure returns (User memory) {
        uint256 x = 1;
        return User(x);
    }
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.is_empty(),
        "GER should skip unsupported Solidity return types: {ger:?}"
    );
}

#[test]
fn ger_does_not_target_existing_return_statements() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function boolFn() public pure returns (bool) {
        ping();
        return false;
    }
}
"#;
    let ger = ger_mutants(source);
    assert!(
        !ger.iter()
            .any(|m| m.old_text.trim_start().starts_with("return ")),
        "GER should not target existing return statements: {ger:?}"
    );
}

#[test]
fn ger_inside_nested_if_block() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function check(uint256 x) public pure returns (uint256) {
        if (x > 0) {
            ping();
        }
        return x;
    }
}
"#;
    let ger = ger_mutants(source);
    assert!(
        !ger.is_empty(),
        "GER should fire on statements inside if blocks: {ger:?}"
    );
}

#[test]
fn ger_with_bytes_return_type() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function getData() public pure returns (bytes32) {
        ping();
        return bytes32(0);
    }
}
"#;
    let ger = ger_mutants(source);
    assert!(
        !ger.is_empty(),
        "GER should handle bytes32 return type: {ger:?}"
    );
    assert!(
        ger.iter().any(|m| m.new_text.contains("bytes32(0)")),
        "GER replacement for bytes32 should use bytes32(0): {ger:?}"
    );
}

#[test]
fn ger_picks_correct_return_type_across_functions() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function getBool() public pure returns (bool) {
        ping();
        return true;
    }

    function getUint() public pure returns (uint256) {
        ping();
        return 42;
    }
}
"#;
    let ger = ger_mutants(source);
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return false;"),
        "getBool should become return false: {ger:?}"
    );
    assert!(
        ger.iter()
            .any(|m| m.old_text.trim() == "ping();" && m.new_text == "return 0;"),
        "getUint should become return 0: {ger:?}"
    );
}
