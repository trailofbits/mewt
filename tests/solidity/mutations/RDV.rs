use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn rdv_replaces_primitive_return_values_with_defaults() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function value() public pure returns (uint256) {
        return 42;
    }

    function flag() public pure returns (bool) {
        return true;
    }

    function owner() public view returns (address) {
        return msg.sender;
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert_eq!(
        mutants.len(),
        3,
        "expected 3 RDV mutants for primitives: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "42" && m.new_text == "0"),
        "RDV should replace uint256 with 0: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "true" && m.new_text == "false"),
        "RDV should replace bool with false: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "msg.sender" && m.new_text == "address(0)"),
        "RDV should replace address with address(0): {mutants:?}"
    );
}

#[test]
fn rdv_handles_payable_addresses_and_signed_integers() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function recipient() public view returns (address payable) {
        return payable(msg.sender);
    }

    function delta() public pure returns (int256) {
        return -42;
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert_eq!(mutants.len(), 2, "expected 2 RDV mutants: {mutants:?}");
    assert!(
        mutants.iter().any(|m| m.new_text == "payable(address(0))"),
        "RDV should replace address payable with payable(address(0)): {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "-42" && m.new_text == "0"),
        "RDV should replace signed integers with 0: {mutants:?}"
    );
}

#[test]
fn rdv_rewrites_string_and_bytes_returns() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function name() public pure returns (string memory) {
        return "hello";
    }

    function data() public pure returns (bytes memory) {
        return hex"abcdef";
    }

    function hash() public pure returns (bytes32) {
        return keccak256("data");
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert_eq!(
        mutants.len(),
        3,
        "expected RDV mutants for string/bytes types: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "\"hello\"" && m.new_text == "\"\""),
        "RDV should replace strings with empty string literals: {mutants:?}"
    );
    assert!(
        mutants.iter().any(|m| m.new_text == "bytes(\"\")"),
        "RDV should replace dynamic bytes with bytes(\"\"): {mutants:?}"
    );
    assert!(
        mutants.iter().any(|m| m.new_text == "bytes32(0)"),
        "RDV should replace bytes32 with bytes32(0): {mutants:?}"
    );
}

#[test]
fn rdv_handles_multiple_return_values_independently() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function info() public pure returns (uint256, bool) {
        return (42, true);
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert_eq!(
        mutants.len(),
        2,
        "expected two RDV mutants for tuple return: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "42" && m.new_text == "0"),
        "Should replace uint256 tuple element with 0: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "true" && m.new_text == "false"),
        "Should replace bool tuple element with false: {mutants:?}"
    );
}

#[test]
fn rdv_skips_non_primitive_return_types() {
    let source = r#"
pragma solidity ^0.8.0;

struct Point { uint256 x; uint256 y; }

contract T {
    enum Status { Active, Inactive }

    mapping(address => uint256) internal balances;

    function origin() public pure returns (Point memory) {
        return Point(0, 0);
    }

    function getStatus() public pure returns (Status) {
        return Status.Active;
    }

    function ids() public pure returns (uint256[] memory) {
        uint256[] memory list = new uint256[](1);
        list[0] = 42;
        return list;
    }

    function ledger() public view returns (mapping(address => uint256) storage) {
        return balances;
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert!(
        mutants.is_empty(),
        "RDV should skip user-defined, enum, array, and mapping returns"
    );
}

#[test]
fn rdv_skips_already_default_values() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function zero() public pure returns (uint256) {
        return 0;
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert!(
        mutants.is_empty(),
        "RDV should not mutate values already at their default"
    );
}

#[test]
fn rdv_skips_named_return_with_implicit_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    mapping(address => uint256) internal balances;

    function balance(address owner) public view returns (uint256 amount) {
        amount = balances[owner];
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert!(
        mutants.is_empty(),
        "RDV should skip implicit returns via named variables"
    );
}

#[test]
fn rdv_mutates_explicit_return_in_named_return_function() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    mapping(address => uint256) internal balances;

    function balance(address owner) public view returns (uint256 amount) {
        amount = balances[owner];
        return amount;
    }
}
"#;
    let mutants = mutants_for_slug(source, "RDV");
    assert_eq!(mutants.len(), 1, "expected exactly one RDV mutant");
    assert_eq!(mutants[0].old_text, "amount");
    assert_eq!(mutants[0].new_text, "0");
}
