use crate::go::integration_tests::create_test_target;
use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;

#[test]
fn cr_wraps_statements_in_block_comments() {
    let source = r#"
package main

func demo() {
    value := compute()
    println(value)
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants: Vec<_> = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "CR")
        .collect();

    assert!(
        !mutants.is_empty(),
        "expected CR mutants to wrap statements"
    );

    for mutant in mutants {
        assert!(mutant.new_text.starts_with("/* ") && mutant.new_text.ends_with(" */"));
    }
}

#[test]
fn cr_covers_simple_statement_forms() {
    let source = r#"
package main

func f() {
    x := 0
    x = 1
    x += 1
    x++
    x--
    _ = x
}
"#;

    let (_tmp, target) = create_test_target(source);
    let mutants = GoLanguageEngine::new()
        .mutate(&target)
        .into_iter()
        .filter(|m| m.mutation_slug == "CR")
        .collect::<Vec<_>>();

    let cases: &[(&str, &str)] = &[
        ("x :=", "short_var_declaration"),
        ("x = 1", "assignment_statement (plain)"),
        ("x +=", "assignment_statement (compound)"),
        ("x++", "inc_statement"),
        ("x--", "dec_statement"),
    ];

    for (prefix, label) in cases {
        assert!(
            mutants
                .iter()
                .any(|m| m.old_text.trim_start().starts_with(prefix)),
            "expected a CR mutant for {} (prefix {:?}); got mutants: {:?}",
            label,
            prefix,
            mutants
                .iter()
                .map(|m| (m.mutation_slug.as_str(), m.old_text.as_str()))
                .collect::<Vec<_>>()
        );
    }
}
