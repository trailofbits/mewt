use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn saos_mutates_shift_assignments() {
    let source = r#"
pragma solidity ^0.8.0;
contract T {
    function f(uint256 a) public pure returns (uint256) {
        a <<= 1;
        return a;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "SAOS", &[">>="]);
}
