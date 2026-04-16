use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn aos_mutates_arithmetic_operators() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function combine(uint256 a, uint256 b) public pure returns (uint256) {
        return a + b;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AOS", &["-", "*", "/"]);
}
