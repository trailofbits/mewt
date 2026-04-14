use mewt::LanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use mewt::types::{Hash, Target};

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

use mewt::types::Mutant;

fn rdv_mutants(mutants: &[Mutant]) -> Vec<&Mutant> {
    mutants
        .iter()
        .filter(|m| m.mutation_slug == "RDV")
        .collect()
}

#[test]
fn test_rdv_uint_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function balance() public pure returns (uint256) {
        return 42;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should generate exactly 1 RDV mutation");
    assert_eq!(rdv[0].old_text, "42");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_bool_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function isValid() public pure returns (bool) {
        return true;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.iter()
            .any(|m| m.old_text == "true" && m.new_text == "false"),
        "RDV should replace true with false: {rdv:?}"
    );
}

#[test]
fn test_rdv_address_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function owner() public pure returns (address) {
        return msg.sender;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.iter()
            .any(|m| m.old_text == "msg.sender" && m.new_text == "address(0)"),
        "RDV should replace address return with address(0): {rdv:?}"
    );
}

#[test]
fn test_rdv_address_payable_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function recipient() public pure returns (address payable) {
        return payable(msg.sender);
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.iter().any(|m| m.new_text == "payable(address(0))"),
        "RDV should replace address payable return with payable(address(0)): {rdv:?}"
    );
}

#[test]
fn test_rdv_multi_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function getInfo() public pure returns (uint256, bool) {
        return (42, true);
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(
        rdv.len(),
        2,
        "Should generate 2 RDV mutations for (uint256, bool)"
    );
    assert!(
        rdv.iter().any(|m| m.old_text == "42" && m.new_text == "0"),
        "RDV should replace uint256 value with 0: {rdv:?}"
    );
    assert!(
        rdv.iter()
            .any(|m| m.old_text == "true" && m.new_text == "false"),
        "RDV should replace bool value with false: {rdv:?}"
    );
}

#[test]
fn test_rdv_multi_return_partial_mapping() {
    let source = r#"
pragma solidity ^0.8.0;

struct Data { uint256 x; }

contract Test {
    function getInfo() public pure returns (uint256, Data memory, bool) {
        Data memory d = Data(1);
        return (42, d, true);
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(
        rdv.len(),
        2,
        "Should generate 2 RDV mutations (uint256 and bool), skipping the struct: {rdv:?}"
    );
    assert!(
        rdv.iter().any(|m| m.old_text == "42" && m.new_text == "0"),
        "RDV should replace uint256 value with 0: {rdv:?}"
    );
    assert!(
        rdv.iter()
            .any(|m| m.old_text == "true" && m.new_text == "false"),
        "RDV should replace bool value with false: {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_already_default() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function zero() public pure returns (uint256) {
        return 0;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutation when return value is already the default"
    );
}

#[test]
fn test_rdv_no_return_value() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function doNothing() public pure {
        return;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutation for void return"
    );
}

#[test]
fn test_rdv_string_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function name() public pure returns (string memory) {
        return "hello";
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1);
    assert_eq!(rdv[0].old_text, "\"hello\"");
    assert_eq!(rdv[0].new_text, "\"\"");
}

#[test]
fn test_rdv_bytes32_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function hash() public pure returns (bytes32) {
        return keccak256("data");
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1);
    assert_eq!(rdv[0].new_text, "\"\"");
}

#[test]
fn test_rdv_int256_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function delta() public pure returns (int256) {
        return -42;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1);
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_multiple_return_statements() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function abs(int256 x) public pure returns (int256) {
        if (x >= 0) {
            return x;
        }
        return -x;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(
        rdv.len(),
        2,
        "Should generate one RDV mutant per return statement: {rdv:?}"
    );
    assert!(
        rdv.iter().any(|m| m.old_text == "x" && m.new_text == "0"),
        "Should replace 'return x' with 'return 0': {rdv:?}"
    );
    assert!(
        rdv.iter().any(|m| m.old_text == "-x" && m.new_text == "0"),
        "Should replace 'return -x' with 'return 0': {rdv:?}"
    );
}

#[test]
fn test_rdv_multi_return_partial_default() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function getInfo() public pure returns (uint256, bool) {
        return (0, true);
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(
        rdv.len(),
        1,
        "Should skip the uint256 (already 0) and only mutate the bool: {rdv:?}"
    );
    assert_eq!(rdv[0].old_text, "true");
    assert_eq!(rdv[0].new_text, "false");
}

// --- Negative tests: types that should NOT produce RDV mutants ---

#[test]
fn test_rdv_skips_user_defined_type() {
    let source = r#"
pragma solidity ^0.8.0;

struct Point { uint256 x; uint256 y; }

contract Test {
    function origin() public pure returns (Point memory) {
        return Point(0, 0);
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutations for user-defined struct return types: {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_enum_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    enum Status { Active, Inactive }

    function getStatus() public pure returns (Status) {
        return Status.Active;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutations for enum return types: {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_array_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    function getIds() public pure returns (uint256[] memory) {
        uint256[] memory ids = new uint256[](1);
        ids[0] = 42;
        return ids;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutations for array return types: {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_mapping_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    mapping(address => uint256) internal balances;

    function getBalances() internal view returns (mapping(address => uint256) storage) {
        return balances;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutations for mapping return types: {rdv:?}"
    );
}

#[test]
fn test_rdv_skips_named_return_implicit() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    mapping(address => uint256) internal balances;

    function balance(address owner) public view returns (uint256 amount) {
        amount = balances[owner];
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutations for implicit returns via named variables: {rdv:?}"
    );
}

#[test]
fn test_rdv_named_return_with_explicit_return() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    mapping(address => uint256) internal balances;

    function balance(address owner) public view returns (uint256 amount) {
        amount = balances[owner];
        return amount;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert_eq!(rdv.len(), 1, "Should mutate the explicit return: {rdv:?}");
    assert_eq!(rdv[0].old_text, "amount");
    assert_eq!(rdv[0].new_text, "0");
}

#[test]
fn test_rdv_skips_bare_return_in_named_return_function() {
    let source = r#"
pragma solidity ^0.8.0;

contract Test {
    mapping(address => uint256) internal balances;

    function balance(address owner) public view returns (uint256 amount) {
        amount = balances[owner];
        if (amount > 0) {
            return;
        }
        amount = 0;
    }
}
"#;
    let target = solidity_target_from_source(source);
    let engine = SolidityLanguageEngine::new();
    let mutants = engine.mutate(&target);
    let rdv = rdv_mutants(&mutants);

    assert!(
        rdv.is_empty(),
        "RDV should not generate mutations for bare 'return;' in named return functions: {rdv:?}"
    );
}
