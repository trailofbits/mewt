use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn cos_mutates_comparison_operators() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function cmp(uint256 a, uint256 b) public pure returns (bool) {
        return a == b;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "COS", &["!=", "<", "<=", ">", ">="]);
}
