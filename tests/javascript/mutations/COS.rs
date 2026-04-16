use crate::javascript::integration_tests::{
    assert_only_slug_and_expected_new_texts, create_test_target,
};
use mewt::LanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;

#[test]
fn cos_mutates_comparison_operators_in_js() {
    let source = r#"
function cmp(a, b) {
  return a == b;
}
"#;

    assert_only_slug_and_expected_new_texts(
        source,
        "test.js",
        "COS",
        &["!=", "===", "!==", "<", "<=", ">", ">="],
    );
}

#[test]
fn cos_ignores_typescript_generics() {
    let source = r#"
// TypeScript generics should NOT be mutated
const emitter = module.get<EventEmitter2>(EventEmitter2);
const result = foo<string, number>(arg1, arg2);

function generic<T>(value: T): T {
    return value;
}

// Real comparisons SHOULD be mutated
if (a < b && c > d) {
    return true;
}

const max = x >= y ? x : y;
"#;
    let (_temp_dir, target) = create_test_target(source, "test.ts");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cos_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    assert!(
        !cos_mutants.is_empty(),
        "Should generate COS mutations for actual comparison operators"
    );

    for mutant in &cos_mutants {
        assert!(
            !mutant.new_text.contains("get<")
                && !mutant.new_text.contains("foo<")
                && !mutant.new_text.contains("generic<"),
            "COS mutation should not mutate TypeScript generics: {}",
            mutant.new_text
        );
    }

    let has_less_than_mutation = cos_mutants.iter().any(|m| m.old_text == "<");
    let has_greater_than_mutation = cos_mutants.iter().any(|m| m.old_text == ">");
    let has_gte_mutation = cos_mutants.iter().any(|m| m.old_text == ">=");

    assert!(
        has_less_than_mutation && has_greater_than_mutation && has_gte_mutation,
        "Should mutate actual comparison operators (<, >, >=) in conditions"
    );
}

#[test]
fn cos_ignores_tsx_jsx_elements() {
    let source = r#"
// TSX with both JSX elements and TypeScript generics
function App<T>(props: { value: T }) {
    return <div>Hello</div>;
}

const result = foo<string, number>(arg1, arg2);
const element = <Component prop="value" />;

// Real comparisons SHOULD be mutated
if (a < b && c > d) {
    return true;
}
"#;
    let (_temp_dir, target) = create_test_target(source, "test.tsx");
    let engine = JavaScriptLanguageEngine::new();
    let mutants = engine.mutate(&target);

    let cos_mutants: Vec<_> = mutants
        .iter()
        .filter(|m| m.mutation_slug == "COS")
        .collect();
    assert!(
        !cos_mutants.is_empty(),
        "Should generate COS mutations for comparison operators in TSX"
    );

    for mutant in &cos_mutants {
        let text = &mutant.new_text;
        assert!(
            !text.contains("<div")
                && !text.contains("</div")
                && !text.contains("<Component")
                && !text.contains("App<")
                && !text.contains("foo<"),
            "COS mutation should not mutate TSX JSX elements or TypeScript generics: {}",
            text
        );
    }

    let has_comparison_mutations = cos_mutants.iter().any(|m| {
        matches!(
            m.old_text.as_str(),
            "<" | "<=" | ">" | ">=" | "==" | "!=" | "===" | "!=="
        )
    });
    assert!(
        has_comparison_mutations,
        "Should mutate actual comparison operators in TSX files"
    );
}
