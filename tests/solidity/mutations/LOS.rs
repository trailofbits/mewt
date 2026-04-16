use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn los_mutates_logical_operators() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function choose(bool a, bool b, bool c) public pure returns (bool) {
        return (a && b) || c;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "LOS", &["&&", "||"]);
}
