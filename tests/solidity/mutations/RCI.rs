use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn rci_inverts_boolean_require_condition() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function transfer(address to, uint256 amount, bool approved) public {
        require(approved, "not approved");
        to.call{value: amount}("");
    }
}
"#;
    let mutants = mutants_for_slug(source, "RCI");
    assert_eq!(mutants.len(), 1, "expected exactly one RCI mutant");
    assert_eq!(mutants[0].new_text, "!(approved)");
}

#[test]
fn rci_inverts_logical_require_condition() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function check(uint256 x, uint256 y) public pure {
        require(x > 0 && y > 0, "invalid");
    }
}
"#;
    let mutants = mutants_for_slug(source, "RCI");
    assert_eq!(mutants.len(), 1);
    assert_eq!(mutants[0].new_text, "!(x > 0 && y > 0)");
}

#[test]
fn rci_skips_simple_comparisons() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function withdraw(uint256 amount) public pure {
        require(amount > 0, "zero amount");
    }
}
"#;
    let mutants = mutants_for_slug(source, "RCI");
    assert!(
        mutants.is_empty(),
        "RCI should defer simple comparisons to COS"
    );
}

#[test]
fn rci_skips_already_negated_conditions() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    bool public paused;

    function check() public view {
        require(!paused, "paused");
    }
}
"#;
    let mutants = mutants_for_slug(source, "RCI");
    assert!(mutants.is_empty(), "RCI should skip !expr conditions");
}

#[test]
fn rci_inverts_function_call_condition() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function isAllowed(address user) internal pure returns (bool) {
        return user != address(0);
    }

    function check(address user) public view {
        require(isAllowed(user), "not allowed");
    }
}
"#;
    let mutants = mutants_for_slug(source, "RCI");
    assert_eq!(mutants.len(), 1);
    assert!(
        mutants[0].new_text.contains("!(isAllowed(user))"),
        "expected RCI to wrap the function call in negation: {mutants:?}"
    );
}
