use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn as_swaps_adjacent_arguments_in_function_calls() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function useIt() public pure returns (uint256) {
        return foo(bar(1, 2, 3));
    }

    function foo(uint256 a, uint256 b, uint256 c) internal pure returns (uint256) {
        return a + b + c;
    }

    function bar(uint256 a, uint256 b, uint256 c) internal pure returns (uint256) {
        return a + b + c;
    }
}
"#;
    let mutants = mutants_for_slug(source, "AS");
    assert!(
        !mutants.is_empty(),
        "expected AS mutants to swap adjacent arguments"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.new_text.contains("2, 1") || m.new_text.contains("3, 2")),
        "expected swapped argument ordering in AS mutants: {mutants:?}"
    );
}
