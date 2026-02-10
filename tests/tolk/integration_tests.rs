use mewt::LanguageEngine;
use mewt::languages::tolk::engine::TolkLanguageEngine;
use mewt::types::Target;
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;

/// Helper to create test target
fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.tolk");
    std::fs::write(&file_path, content).expect("Failed to write test file");
    let target = Target {
        id: 1,
        path: file_path,
        file_hash: mewt::types::Hash::digest(content.to_string()),
        text: content.to_string(),
        language: "Tolk".to_string(),
    };
    (temp_dir, target)
}

#[test]
fn test_mutation_count_comparison() {
    let source = r#"struct Storage {
    counter: uint64
}

fun Storage.load() {
    return Storage.fromCell(contract.getData());
}

fun Storage.save(self) {
    contract.setData(self.toCell());
}

fun onInternalMessage(in: InMessage) {
    var storage = lazy Storage.load();
    if (in.valueCoins < 1000) {
        throw 100;
    }
    storage.counter += 1;
    storage.save();
}

get fun currentCounter(): int {
    val storage = lazy Storage.load();
    return storage.counter;
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    let ast_engine = TolkLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    println!("AST mutations: {}", ast_mutants.len());

    assert!(
        !ast_mutants.is_empty(),
        "AST should generate some mutations"
    );

    let ast_slugs: HashSet<_> = ast_mutants
        .iter()
        .map(|m| m.mutation_slug.chars().take(2).collect::<String>())
        .collect();

    println!("AST mutation types: {ast_slugs:?}");

    assert!(
        ast_slugs.len() > 1,
        "AST should generate diverse mutation types"
    );
}

#[test]
fn test_mutation_quality_comparison() {
    let source = r#"fun onInternalMessage(in: InMessage) {
    // Verify sender has enough balance
    if (in.valueCoins > 0) {
        return;
    }
    throw 100;
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    let ast_engine = TolkLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    let ast_comment_mutations = ast_mutants
        .iter()
        .filter(|m| m.old_text.trim().starts_with("//"))
        .count();

    println!("AST comment mutations: {ast_comment_mutations}");

    assert_eq!(
        ast_comment_mutations, 0,
        "AST should not mutate comment-only lines"
    );
}

#[test]
fn test_complex_code_handling() {
    let source = r#"const MAX_SUPPLY = 1000000000;

struct Storage {
    totalSupply: coins,
    adminAddress: address,
    paused: bool
}

fun Storage.load() {
    return Storage.fromCell(contract.getData());
}

fun Storage.save(self) {
    contract.setData(self.toCell());
}

struct(0x7e8764ef) MintTokens {
    amount: coins,
    recipient: address
}

struct(0x595f07bc) BurnNotification {
    amount: coins,
    senderAddress: address
}

type AllowedMessage = MintTokens | BurnNotification;

fun onInternalMessage(in: InMessage) {
    val msg = lazy AllowedMessage.fromSlice(in.body);
    var storage = lazy Storage.load();

    match (msg) {
        MintTokens => {
            assert(storage.paused == false) throw 403;
            val newSupply: coins = storage.totalSupply + msg.amount;
            assert(newSupply <= MAX_SUPPLY) throw 404;
            storage.totalSupply = newSupply;
            storage.save();
        }

        BurnNotification => {
            assert(msg.amount > 0) throw 400;
            if (storage.totalSupply >= msg.amount) {
                storage.totalSupply -= msg.amount;
            }
            storage.save();
        }

        else => {
            assert(in.body.isEmpty()) throw 0xFFFF;
        }
    }
}

fun computeFee(amount: coins, basisPoints: int): coins {
    val fee: coins = amount * basisPoints / 10000;
    if (fee < 1) {
        return 1;
    }
    return fee;
}

fun pow(base: int, exp: int): int {
    var result: int = 1;
    var i: int = 0;
    while (i < exp) {
        result *= base;
        i += 1;
    }
    return result;
}

get fun totalSupply(): coins {
    val storage = lazy Storage.load();
    return storage.totalSupply;
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    let ast_engine = TolkLanguageEngine::new();
    let ast_result = std::panic::catch_unwind(|| ast_engine.apply_all_mutations(&target));

    assert!(
        ast_result.is_ok(),
        "AST system should handle complex code without panicking"
    );

    if let Ok(ast_mutants) = ast_result {
        println!("Complex code - AST mutations: {}", ast_mutants.len());

        assert!(
            ast_mutants.len() > 5,
            "AST should generate substantial mutations for complex code"
        );
    }
}

#[test]
fn test_mutation_overlap_analysis() {
    let source = r#"fun computeFee(amount: coins, basisPoints: int): coins {
    val fee: coins = amount * basisPoints / 10000;
    if (fee < 1) {
        return 1;
    }
    return fee;
}
"#;

    let (_temp_dir, target) = create_test_target(source);

    let ast_engine = TolkLanguageEngine::new();
    let ast_mutants = ast_engine.apply_all_mutations(&target);

    let mut ast_lines: HashMap<usize, Vec<String>> = HashMap::new();

    for mutant in &ast_mutants {
        ast_lines
            .entry(mutant.line_offset as usize)
            .or_default()
            .push(mutant.mutation_slug.clone());
    }

    println!("AST mutations by line: {ast_lines:?}");

    assert!(
        ast_lines.len() > 1,
        "AST mutations should affect multiple lines"
    );
}
