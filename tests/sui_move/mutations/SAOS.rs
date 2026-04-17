use crate::sui_move::integration_tests::mutants_for_slug;

#[test]
fn saos_mutation_is_not_generated_in_sui_move() {
    let source = r#"module test::m {
    fun f(a: u64): u64 {
        a << 1
    }
}"#;

    let mutants = mutants_for_slug(source, "SAOS");
    assert!(
        mutants.is_empty(),
        "Sui Move should not produce SAOS mutants, found: {mutants:?}"
    );
}
