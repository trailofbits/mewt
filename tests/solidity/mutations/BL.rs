use crate::solidity::integration_tests::assert_only_slug_and_expected_new_texts;

#[test]
fn bl_flips_true_to_false() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function ready() public pure returns (bool) {
        return true;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BL", &["false"]);
}

#[test]
fn bl_flips_false_to_true() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function paused() public pure returns (bool) {
        return false;
    }
}
"#;

    assert_only_slug_and_expected_new_texts(source, "BL", &["true"]);
}
