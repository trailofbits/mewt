use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn er_replaces_statements_with_require_false() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function maybeAdd(uint256 x) public pure returns (uint256) {
        if (x > 0) {
            return x + 1;
        }
        return x - 1;
    }
}
"#;
    let mutants = mutants_for_slug(source, "ER");
    assert!(
        !mutants.is_empty(),
        "expected ER to replace executable statements"
    );
    assert!(
        mutants
            .iter()
            .all(|m| m.new_text.trim() == "require(false);"),
        "ER mutants should emit require(false);: {mutants:?}"
    );
}

#[test]
fn er_skips_statements_with_existing_require_call() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function check(uint256 x) public pure {
        require(x > 0, "x must be positive");
    }
}
"#;
    let mutants = mutants_for_slug(source, "ER");
    assert!(
        mutants.is_empty(),
        "ER should skip statements that already contain require(): {mutants:?}"
    );
}
