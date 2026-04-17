use crate::sui_move::integration_tests::mutants_for_slug;

#[test]
fn er_replaces_statements_with_abort() {
    let source = r#"module test::m {
    fun maybe_add(a: u64, b: u64): u64 {
        let sum = a + b;
        sum
    }
}"#;

    let mutants = mutants_for_slug(source, "ER");
    assert!(
        !mutants.is_empty(),
        "expected ER mutants to replace statements"
    );

    assert!(
        mutants.iter().all(|m| m.new_text == "abort 0;"),
        "ER mutants should replace statements with `abort 0;`: {mutants:?}"
    );
}

#[test]
fn er_does_not_replace_existing_abort_statements() {
    let source = r#"module test::m {
    fun safe(b: u64): u64 {
        if (b == 0) { abort 0 };
        42 / b
    }
}"#;

    let mutants = mutants_for_slug(source, "ER");
    assert!(
        mutants.iter().all(|m| !m.old_text.contains("abort ")),
        "ER should not replace existing abort statements: {mutants:?}"
    );
}
