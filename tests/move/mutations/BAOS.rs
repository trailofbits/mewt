use crate::r#move::shared::mutants_for_slug;

#[test]
fn baos_mutation_is_not_generated_in_sui_move() {
    let source = r#"module test::m {
    fun f(a: u64, b: u64): u64 {
        a & b
    }
}"#;

    let mutants = mutants_for_slug(source, "move/sui", "BAOS");
    assert!(
        mutants.is_empty(),
        "Sui Move should not produce BAOS mutants, found: {mutants:?}"
    );
}
