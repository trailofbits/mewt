use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn cr_wraps_statements_in_block_comments() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function maybe(uint256 value) public pure returns (uint256) {
        if (value > 0) {
            return value;
        }
        return 0;
    }
}
"#;
    let mutants = mutants_for_slug(source, "CR");
    assert!(!mutants.is_empty(), "expected CR mutants");
    assert!(
        mutants
            .iter()
            .all(|m| m.new_text.trim().starts_with("/*") && m.new_text.trim().ends_with("*/")),
        "CR mutants should wrap statements in comments: {mutants:?}"
    );
}
