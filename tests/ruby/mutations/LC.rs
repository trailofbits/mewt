use crate::ruby::integration_tests::create_test_target;
use crate::utils::mutants_for_slug;
use mewt::languages::ruby::engine::RubyLanguageEngine;

#[test]
fn lc_swaps_break_and_next() {
    let source = r#"
xs.each do |x|
  break if x < 0
  next if x == 0
  puts x
end
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "LC");

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "break" && m.new_text == "next"),
        "expected LC to turn break into next: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "break" && m.new_text == "redo"),
        "expected LC to turn break into redo: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "break" && m.new_text == "retry"),
        "expected LC to turn break into retry: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "next" && m.new_text == "break"),
        "expected LC to turn next into break: {mutants:?}"
    );
}

#[test]
fn lc_swaps_redo_and_retry() {
    let source = r#"
xs.each do |x|
  redo if x < 0
  retry if x == 0
  puts x
end
"#;
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = mutants_for_slug(&engine, &target, "LC");

    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "redo" && m.new_text == "next"),
        "expected LC to turn redo into next: {mutants:?}"
    );
    assert!(
        mutants
            .iter()
            .any(|m| m.old_text.trim() == "retry" && m.new_text == "break"),
        "expected LC to turn retry into break: {mutants:?}"
    );
}
