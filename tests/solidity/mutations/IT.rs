use crate::solidity::integration_tests::mutants_for_slug;

#[test]
fn it_mutation_forces_condition_to_true() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function shouldRetry(uint256 attempts) public pure returns (bool) {
        if (attempts == 0) {
            return false;
        }
        return true;
    }
}
"#;
    let mutants = mutants_for_slug(source, "IT");
    assert!(
        mutants
            .iter()
            .any(|m| m.new_text.trim() == "true" || m.new_text.trim() == "(true)"),
        "expected IT to replace condition with true: {mutants:?}"
    );
}

#[test]
fn it_mutation_preserves_parentheses_for_complex_conditions() {
    let source = r#"
pragma solidity ^0.8.0;

contract T {
    function guarded(bool ready, bool allowed) public pure {
        if (ready && allowed) {
            return;
        }
    }
}
"#;
    let mutants = mutants_for_slug(source, "IT");
    assert!(
        mutants.iter().any(|m| {
            m.old_text.trim() == "ready && allowed"
                && (m.new_text.trim() == "true" || m.new_text.trim() == "(true)")
        }),
        "expected IT mutant to replace complex condition with true while preserving grouping: {mutants:?}"
    );
}
