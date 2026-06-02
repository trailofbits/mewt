use crate::daml::integration_tests::mutants_for_slug;
use std::collections::HashSet;

#[test]
fn cos_shuffles_comparison_operators() {
    let source = r#"module M where

f : Int -> Bool
f x = x == 0
"#;
    // `==` is replaced by each other comparison operator. Note DAML spells
    // inequality `/=`, not `!=`. We assert the exact new_text set rather than
    // substrings, so a `<=` cannot satisfy a `<` expectation.
    let cos = mutants_for_slug(source, "COS");
    assert_eq!(
        cos.len(),
        5,
        "COS should produce 5 replacements for ==: {cos:?}"
    );
    assert!(
        cos.iter().all(|m| m.old_text == "=="),
        "all COS mutants should replace ==: {cos:?}"
    );
    let new_ops: HashSet<&str> = cos.iter().map(|m| m.new_text.as_str()).collect();
    assert_eq!(
        new_ops,
        ["/=", "<", "<=", ">", ">="]
            .into_iter()
            .collect::<HashSet<_>>(),
        "== should be replaced with every other comparison operator"
    );
}
