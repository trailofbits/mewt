use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn aaos_mutates_arithmetic_assignments() {
    let source = r#"
pragma solidity ^0.8.0;
contract T {
    function f(uint256 a, uint256 b) public pure returns (uint256) {
        a += b;
        return a;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "AAOS", &["-=", "*=", "/=", "%="]);
}
