use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bos_mutates_bitwise_operators() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function mask(uint256 flags, uint256 mask) public pure returns (uint256) {
        return flags & mask;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BOS", &["|", "^"]);
}
