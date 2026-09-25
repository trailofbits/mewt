use crate::ruby::integration_tests::assert_only_slug_and_expected_new_texts;
use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn cr_replaces_statements_with_nil() {
    // CR replaces statement-position nodes with `nil`. This is always syntactically
    // valid regardless of indentation level (unlike `=begin/=end` block comments
    // which require column 0 placement).
    let source = r#"
puts "hello"
"#;
    assert_only_slug_and_expected_new_texts(source, "CR", &["nil"]);
}

#[test]
fn cr_covers_broad_statement_kinds() {
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
    assert_only_slug_and_expected_new_texts(source, "CR", &["nil"]);
}

#[test]
fn cr_does_not_replace_call_in_expression_position() {
    // CR should not produce a mutant for a bare call sub-expression inside a
    // binary expression — the enclosing assignment statement is already a target.
    let source = r#"
total = a + compute(b)
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "CR");

    let bare_call_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.old_text.trim() == "compute(b)")
        .collect();
    assert!(
        bare_call_mutants.is_empty(),
        "CR should not target a bare call sub-expression inside a binary; got {bare_call_mutants:?}"
    );
}

#[test]
fn cr_does_not_replace_call_in_string_interpolation() {
    let source = r#"
msg = "result is #{compute(x)}"
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "CR");

    let interp_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.old_text.trim() == "compute(x)")
        .collect();
    assert!(
        interp_mutants.is_empty(),
        "CR should not target a call inside string interpolation; got {interp_mutants:?}"
    );
}
