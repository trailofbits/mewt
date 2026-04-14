use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::{Hash, Mutant, Target};

fn solidity_target_from_source(source: &str) -> Target {
    use tempfile::tempdir;
    let tmp = tempdir().expect("tmpdir");
    let path = tmp.path().join("test.sol");
    std::fs::write(&path, source).unwrap();
    Target {
        id: 1,
        path,
        file_hash: Hash::digest(source.to_string()),
        text: source.to_string(),
        language: "Solidity".to_string(),
    }
}

#[test]
fn solidity_shared_slugs_presence() {
    // Solidity sample with if and a call with 2 args
    let solidity_src = r#"
pragma solidity ^0.8.0;

contract Test {
    function main() public {
        uint256 x = 1;
        if (x > 0) {
            return;
        }
        doSomething(1, 2);
    }
    
    function doSomething(uint256 a, uint256 b) public {}
}
"#;

    let target = solidity_target_from_source(solidity_src);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    fn count(mutants: &[mewt::types::Mutant], slug: &str) -> usize {
        mutants.iter().filter(|m| m.mutation_slug == slug).count()
    }

    let er_count = count(&mutants, "ER");
    let cr_count = count(&mutants, "CR");
    let as_count = count(&mutants, "AS");

    println!("solidity ER/CR/AS: {er_count}/{cr_count}/{as_count}");

    assert!(er_count > 0, "ER should be present in Solidity");
    assert!(cr_count > 0, "CR should be present in Solidity");
    // AS may or may not be present depending on implementation
}

#[test]
fn test_error_replacement_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 42;
        if (x > 0) {
            return x + 1;
        }
        return x - 1;
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let er_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "ER").collect();

    assert!(!er_mutants.is_empty(), "Should generate ER mutations");

    // Check that ER mutations replace expressions with revert calls
    for mutant in er_mutants {
        assert!(
            mutant.new_text.contains("revert(") || mutant.new_text.contains("require(false"),
            "ER mutation should contain revert or require(false) call: {}",
            mutant.new_text
        );
    }
}

#[test]
fn test_comment_replacement_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 42;
        if (x > 0) {
            return x;
        }
        return 0;
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cr_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "CR").collect();

    assert!(!cr_mutants.is_empty(), "Should generate CR mutations");

    // Check that CR mutations wrap code in comments
    for mutant in cr_mutants {
        assert!(
            mutant.new_text.starts_with("/*") && mutant.new_text.ends_with("*/"),
            "CR mutation should wrap in block comments: {}",
            mutant.new_text
        );
    }
}

#[test]
fn test_conditional_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 42;
        if (x > 0) {
            return x;
        } else {
            return 0;
        }
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Should have mutations that target conditional expressions
    let conditional_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.old_text.contains(">") || m.old_text.contains("if"))
        .collect();

    assert!(
        !conditional_mutants.is_empty(),
        "Should generate conditional mutations"
    );
}

#[test]
fn test_argument_swap_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public {
        foo(1, 2);
        bar(x, y, z);
    }
    
    function foo(uint256 a, uint256 b) public {}
    function bar(uint256 x, uint256 y, uint256 z) public {}
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let as_mutants: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == "AS").collect();

    // AS mutations may or may not be present depending on implementation
    if !as_mutants.is_empty() {
        // If AS mutations exist, they should swap function arguments
        for mutant in as_mutants {
            assert!(
                mutant.old_text.contains("(") && mutant.old_text.contains(")"),
                "AS mutation should involve function call: {}",
                mutant.old_text
            );
        }
    }
}

#[test]
fn test_variable_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 x = 1;
        uint256 y = 2;
        return x + y;
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Should have mutations that target variables and expressions
    let var_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| {
            m.old_text.trim() == "x" || m.old_text.trim() == "y" || m.old_text.contains("+")
        })
        .collect();

    assert!(
        !var_mutants.is_empty(),
        "Should generate variable-related mutations"
    );
}

#[test]
fn test_loop_mutations() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function testFunc() public pure returns (uint256) {
        uint256 i = 0;
        while (i < 10) {
            i += 1;
        }
        return i;
    }
}
"#;

    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);

    // Should have mutations that target loop constructs
    let loop_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| {
            m.old_text.contains("while") || m.old_text.contains("<") || m.old_text.contains("+=")
        })
        .collect();

    assert!(
        !loop_mutants.is_empty(),
        "Should generate loop-related mutations"
    );
}

#[test]
fn compound_assignment_slugs_produce_mutants() {
    // Regression test for .todo/a3c12f04: AAOS/BAOS/SAOS were wired to
    // `binary_expression`, but compound assignment in tree-sitter-solidity
    // parses as `augmented_assignment_expression`. The slugs silently emitted
    // zero mutants.
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function f() public {
        uint256 x = 1;
        x += 1;
        x -= 1;
        x *= 2;
        x /= 2;
        x %= 2;
        x &= 1;
        x |= 1;
        x <<= 1;
        x >>= 1;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let mutants = SolidityLanguageEngine::new().mutate(&target);
    let slugs: std::collections::HashSet<_> =
        mutants.iter().map(|m| m.mutation_slug.as_str()).collect();
    for slug in ["AAOS", "BAOS", "SAOS"] {
        assert!(
            slugs.contains(slug),
            "expected slug {} to produce at least one mutant; got slugs: {:?}",
            slug,
            slugs
        );
    }
    // Verify `%=` is covered in AAOS
    assert!(
        mutants
            .iter()
            .any(|m| m.mutation_slug == "AAOS" && m.old_text == "%="),
        "expected an AAOS mutant with old_text `%=`"
    );
}

fn nr_mutants(mutants: &[Mutant]) -> Vec<&Mutant> {
    mutants.iter().filter(|m| m.mutation_slug == "NR").collect()
}

#[test]
fn test_negation_removal_basic() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    bool public paused;

    function check() public view {
        require(!paused, "paused");
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert_eq!(nr.len(), 1, "Should generate exactly 1 NR mutation");
    assert_eq!(nr[0].old_text, "!paused");
    assert_eq!(nr[0].new_text, "paused");
}

#[test]
fn test_negation_removal_complex_expression() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function check(bool a, bool b) public pure {
        require(!(a && b), "both true");
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert!(
        nr.iter()
            .any(|m| m.old_text == "!(a && b)" && m.new_text == "(a && b)"),
        "NR should remove negation preserving parenthesized operand: {nr:?}"
    );
}

#[test]
fn test_negation_removal_ignores_other_unary_ops() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function check(uint256 x) public pure returns (uint256) {
        return ~x;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert!(nr.is_empty(), "NR should not trigger on ~ unary operator");
}

#[test]
fn test_negation_removal_in_comment_ignored() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    // require(!paused);
    /* !flag */
    function f() public {}
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let nr = nr_mutants(&mutants);

    assert!(
        nr.is_empty(),
        "NR should not generate mutations inside comments"
    );
}
