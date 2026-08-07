use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;
use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn er_replaces_statements_with_raise() {
    let source = r#"
puts "hello"
"#;
    assert_only_slug_and_expected_new_texts(source, "ER", &["raise \"mewt\""]);
}

#[test]
fn er_covers_broad_statement_kinds() {
    let source = r#"
result = value > 0 ? 1 : 2
unless done
  work()
end
case value
when 1
  handle()
end
case value
in [1, 2]
  handle()
end
yield 1
super
do_something rescue nil
loop do
  break
  next
  redo
  retry
end
"#;
    assert_only_slug_and_expected_new_texts(source, "ER", &["raise \"mewt\""]);
}

#[test]
fn er_targets_inner_statements_not_method_body() {
    let source = r#"
def process
  step_one()
  step_two()
end
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "ER");

    assert_eq!(
        mutants.len(),
        2,
        "expected one ER mutant per inner statement, got {mutants:?}"
    );
    assert!(
        mutants.iter().all(|m| m.new_text == "raise \"mewt\""),
        "ER should replace each statement with raise"
    );
}

#[test]
fn er_does_not_replace_call_in_expression_position() {
    // A call on the RHS of a binary expression is in expression position.
    // ER should not produce a mutant where old_text == "compute(b)" (the bare call),
    // only a mutant replacing the whole assignment statement is valid.
    let source = r#"
total = a + compute(b)
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "ER");

    let bare_call_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.old_text.trim() == "compute(b)")
        .collect();
    assert!(
        bare_call_mutants.is_empty(),
        "ER should not replace the bare call sub-expression, only the enclosing statement; got {bare_call_mutants:?}"
    );
}

#[test]
fn er_does_not_replace_call_in_string_interpolation() {
    let source = r#"
msg = "result is #{compute(x)}"
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "ER");

    let interp_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.old_text.trim() == "compute(x)")
        .collect();
    assert!(
        interp_mutants.is_empty(),
        "ER should not replace a call inside string interpolation as a standalone mutant, got {interp_mutants:?}"
    );
}
