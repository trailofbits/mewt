use crate::conformance;
use crate::utils;
use mewt::languages::daml::engine::DamlLanguageEngine;
use mewt::types::{Mutant, Target};
use tempfile::TempDir;

/// Build a temporary `test.daml` file from the given source and return both
/// the temp dir (keep it alive for the duration of the test) and the
/// `Target` mewt will mutate. Mirrors `tests/rust/integration_tests.rs`.
pub(crate) fn create_test_target(content: &str) -> (TempDir, Target) {
    utils::target_fixture_for_extension("DAML", "daml", content).into_parts()
}

/// Collect the mutants for a single slug from an inline source. Mirrors how
/// `tests/cpp/integration_tests.rs` exposes `mutants_for_slug` so the per-slug
/// modules can make exact count / old_text / new_text assertions.
pub(crate) fn mutants_for_slug(source: &str, slug: &str) -> Vec<Mutant> {
    let (_tmp, target) = create_test_target(source);
    utils::mutants_for_slug(&DamlLanguageEngine::new(), &target, slug)
}

/// Assert the mutants for `slug` produce exactly the expected replacement texts.
pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    utils::assert_only_slug_and_expected_new_texts(
        &DamlLanguageEngine::new(),
        &target,
        slug,
        expected_new_texts,
    );
}

#[test]
fn daml_common_conformance_checks() {
    // The DAML constructs mewt mutates (if-expressions, infix operators, boolean
    // constructors) live in plain top-level functions; template/choice bodies
    // are handled separately by the CPS/CPR suites.
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"module M where

f : Int -> Int
f x = if x > 0 then x * 2 else x - 1

g : Bool -> Bool -> Bool
g a b = a && b
"#,
        comment_source: r#"module M where

-- this comment must never be mutated
f : Int -> Int
f x = x + 1
"#,
        complex_source: r#"module M where

classify : Int -> Int -> Int
classify x y =
  if x > 0 && y > 0
    then x + y
    else x - y

ready : Bool
ready = True
"#,
        line_coverage_source: r#"module M where

f : Int -> Int
f x = x + 1

g : Int -> Int
g y = y * 2
"#,
    };
    let expectations = conformance::CommonConformanceExpectations {
        language_name: "DAML",
        min_complex_mutants: 12,
    };
    conformance::run_common_language_checks(
        create_test_target,
        || Box::new(DamlLanguageEngine::new()),
        sources,
        expectations,
    );
}

#[test]
fn example_file_exercises_in_scope_mutations() {
    // Asserts the three DAML-specific mutations (CPS swap a party from
    // `controller owner, custodian`, CPR drop a party from the same list,
    // BL flip `isOpen = True`) fire on the canonical corpus; other operators
    // are covered by `daml_common_conformance_checks` and
    // `infix_shufflers_do_not_cross_contaminate`.
    let source = include_str!("example.daml");
    assert!(
        !mutants_for_slug(source, "CPS").is_empty(),
        "example.daml should produce at least one CPS mutant"
    );
    assert!(
        !mutants_for_slug(source, "CPR").is_empty(),
        "example.daml should produce at least one CPR mutant"
    );
    assert!(
        !mutants_for_slug(source, "BL").is_empty(),
        "example.daml should produce at least one BL mutant"
    );
}

// DAML comments use `--` (line) and `{- -}` (block), which the shared
// conformance harness (built around C-style `//`) does not check. Cover the
// three comment shapes explicitly here.
#[test]
fn comment_lines_are_not_mutated() {
    // A commented `True` plus a live `True`. Only the live one may be counted.
    let source = r#"module M where

-- ready = True
f : Bool
f = True
"#;
    let mutants = mutants_for_slug(source, "BL");
    assert_eq!(
        mutants.len(),
        1,
        "only the live `f = True` should be a BL mutant; got {mutants:?}"
    );
    assert_eq!(
        mutants[0].old_text, "True",
        "the single BL mutant must be the live `True`, not the commented one"
    );
}

#[test]
fn block_comments_are_not_mutated() {
    // A `{- ... -}` block comment containing `True` plus a live `True`.
    let source = r#"module M where

{- ready = True -}
f : Bool
f = True
"#;
    let mutants = mutants_for_slug(source, "BL");
    assert_eq!(
        mutants.len(),
        1,
        "only the live `f = True` should be a BL mutant; got {mutants:?}"
    );
    assert_eq!(mutants[0].old_text, "True");
}

#[test]
fn inline_trailing_comments_are_not_mutated() {
    // The trailing comment repeats the live `True`. If comment-skipping
    // regressed the count would be 2; intact, only the live `True` flips.
    let source = r#"module M where

f : Bool
f = True  -- we keep this True
"#;
    let mutants = mutants_for_slug(source, "BL");
    assert_eq!(
        mutants.len(),
        1,
        "only the live `True` should be a BL mutant; got {mutants:?}"
    );
    assert_eq!(mutants[0].old_text, "True");
}

// The three infix shufflers (COS, LOS, AOS) all key off the same `infix` node
// kind. This proves they stay in their lanes: each fires on a source mixing all
// three operator classes, and no class leaks into another slug.
#[test]
fn infix_shufflers_do_not_cross_contaminate() {
    let source = r#"module M where

classify : Int -> Int -> Int
classify x y = if x == 0 && y + 1 > 0 then x else y
"#;
    let cos = mutants_for_slug(source, "COS");
    let los = mutants_for_slug(source, "LOS");
    let aos = mutants_for_slug(source, "AOS");

    assert!(!cos.is_empty(), "COS should fire at least once");
    assert!(!los.is_empty(), "LOS should fire at least once");
    assert!(!aos.is_empty(), "AOS should fire at least once");

    let comparison = ["==", "/=", "<", "<=", ">", ">="];
    let logical = ["&&", "||"];
    let arithmetic = ["+", "-", "*", "/"];

    for m in &cos {
        assert!(
            comparison.contains(&m.old_text.as_str()),
            "COS old_text leaked a non-comparison operator: {:?}",
            m.old_text
        );
        assert!(
            comparison.contains(&m.new_text.as_str()),
            "COS new_text leaked a non-comparison operator: {:?}",
            m.new_text
        );
    }
    for m in &los {
        assert!(
            logical.contains(&m.old_text.as_str()),
            "LOS old_text leaked a non-logical operator: {:?}",
            m.old_text
        );
        assert!(
            logical.contains(&m.new_text.as_str()),
            "LOS new_text leaked a non-logical operator: {:?}",
            m.new_text
        );
    }
    for m in &aos {
        assert!(
            arithmetic.contains(&m.old_text.as_str()),
            "AOS old_text leaked a non-arithmetic operator: {:?}",
            m.old_text
        );
        assert!(
            arithmetic.contains(&m.new_text.as_str()),
            "AOS new_text leaked a non-arithmetic operator: {:?}",
            m.new_text
        );
    }
}
