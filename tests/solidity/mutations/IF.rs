use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn if_mutation_forces_condition_to_false() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function check(uint256 a, uint256 b) public pure returns (bool) {
        if (a > b) {
            return true;
        }
        return false;
    }
}
"#;
    let mutants = mutants_for_slug(source, "IF");
    assert!(
        mutants
            .iter()
            .any(|m| m.new_text.trim() == "false" || m.new_text.trim() == "(false)"),
        "expected IF to replace condition with false: {mutants:?}"
    );
}

#[test]
fn if_mutation_preserves_parentheses_for_complex_conditions() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function guarded(bool ok, bool ready) public pure {
        if (ok && ready) {
            revert();
        }
    }
}
"#;
    let mutants = mutants_for_slug(source, "IF");
    assert!(
        mutants.iter().any(|m| {
            m.old_text.trim() == "ok && ready"
                && (m.new_text.trim() == "false" || m.new_text.trim() == "(false)")
        }),
        "expected IF mutant to replace complex condition with false while preserving grouping: {mutants:?}"
    );
}
