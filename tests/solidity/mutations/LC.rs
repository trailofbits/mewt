use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn lc_swaps_loop_control_keywords() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function iterate(uint256[] memory values) public pure returns (uint256) {
        uint256 count = 0;
        for (uint256 i = 0; i < values.length; i++) {
            if (values[i] == 0) {
                break;
            }
            if (values[i] == 1) {
                continue;
            }
            count += 1;
        }
        return count;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "LC", &["break", "continue"]);
}
