use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn sos_mutates_shift_operators() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function shift(uint256 value) public pure returns (uint256) {
        return value << 2;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "SOS", &[">>"]);
}

#[test]
fn sos_mutates_right_shift_to_left_shift() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function rotate(uint256 value, uint256 amount) public pure returns (uint256) {
        return value >> amount;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "SOS", &["<<"]);
}
