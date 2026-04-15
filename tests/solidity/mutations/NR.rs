use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn nr_removes_negation_operator() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    bool public paused;

    function check() public view {
        require(!paused, "paused");
    }
}
"#;
    let mutants = mutants_for_slug(source, "NR");
    assert_eq!(mutants.len(), 1, "Should generate exactly one NR mutant");
    assert_eq!(mutants[0].old_text, "!paused");
    assert_eq!(mutants[0].new_text, "paused");
}

#[test]
fn nr_removes_negation_from_parenthesized_expression() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function check(bool a, bool b) public pure {
        require(!(a && b), "both true");
    }
}
"#;
    let mutants = mutants_for_slug(source, "NR");
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text == "!(a && b)" && m.new_text == "(a && b)"),
        "NR should remove negation while preserving operand: {mutants:?}"
    );
}

#[test]
fn nr_ignores_other_unary_operators() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function invert(uint256 x) public pure returns (uint256) {
        return ~x;
    }
}
"#;
    let mutants = mutants_for_slug(source, "NR");
    assert!(mutants.is_empty(), "NR should not target ~ unary operator");
}

#[test]
fn nr_ignores_negation_inside_comments() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    // require(!paused);
    /* !flag */
    function noop() public pure {}
}
"#;
    let mutants = mutants_for_slug(source, "NR");
    assert!(
        mutants.is_empty(),
        "NR should not generate mutations inside comments"
    );
}
