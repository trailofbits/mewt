use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn wf_replaces_while_condition_with_false() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function drain(uint256 target) public pure returns (uint256) {
        uint256 sum = 0;
        while (sum < target) {
            sum += 1;
        }
        return sum;
    }
}
"#;
    let mutants = mutants_for_slug(source, "WF");
    assert!(
        mutants
            .iter()
            .any(|m| m.new_text.trim() == "false" || m.new_text.trim() == "(false)"),
        "expected WF to replace while condition with false: {mutants:?}"
    );
}

#[test]
fn wf_preserves_parentheses_for_complex_conditions() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function drain(uint256[] memory queue) public pure returns (uint256) {
        uint256 processed = 0;
        while (processed < queue.length && queue[processed] != 0) {
            processed++;
        }
        return processed;
    }
}
"#;
    let mutants = mutants_for_slug(source, "WF");
    assert!(
        mutants.iter().any(|m| m.old_text.trim()
            == "(processed < queue.length && queue[processed] != 0)"
            && m.new_text == "(false)"),
        "expected WF to retain parentheses in replacement: {mutants:?}"
    );
}
