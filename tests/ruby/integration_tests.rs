use crate::conformance;
use crate::utils;
use mewt::LanguageEngine;
use mewt::languages::ruby::engine::RubyLanguageEngine;
use mewt::types::Target;

pub(crate) fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    utils::target_fixture_for_extension("ruby", "rb", content).into_parts()
}

#[test]
fn ruby_common_conformance_checks() {
    let sources = conformance::CommonConformanceSources {
        basic_source: r#"
def test_func
  x = 42
  if x > 0
    puts "positive"
  end
  y = x + 1
  y
end
"#,
        comment_source: r#"
def test_func
  # This is a comment
  x = 42
  if x > 0
    puts "positive"
  end
  y = x + 1
  y
end
"#,
        complex_source: r#"
def process(data)
  total = 0
  data.each do |value|
    next if value < 0
    total += value
  end

  if total > 0
    total *= 2
  end

  total
end
"#,
        line_coverage_source: r#"
def compute(value)
  if value > 0
    value += 1
  else
    value -= 1
  end

  while value.abs > 0
    value -= 1
  end

  value
end
"#,
    };

    let expectations = conformance::CommonConformanceExpectations {
        language_name: "ruby",
        min_complex_mutants: 6,
    };

    conformance::run_common_language_checks(
        create_test_target,
        || Box::new(RubyLanguageEngine::new()),
        sources,
        expectations,
    );
}

#[test]
fn ruby_example_file_generates_mutants() {
    let source = conformance::read_example_source("tests/ruby/example.rb");
    let (_tmp, target) = create_test_target(&source);
    let mutants = RubyLanguageEngine::new().mutate(&target);

    assert!(
        !mutants.is_empty(),
        "Ruby example file should generate mutants"
    );
}

#[test]
fn ruby_mutations_ignore_comment_lines() {
    let source = r#"
# if true
# x = 1
# while false
# next

def test_comment_region
  value = 1
  if value > 0
    return value
  end
end
"#;

    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = engine.mutate(&target);

    for mutant in mutants {
        assert!(
            !mutant.old_text.trim_start().starts_with('#'),
            "Ruby mutations should not target comment-only lines",
        );
    }
}

#[test]
fn ruby_er_targets_inner_statements_not_method_body() {
    let source = r#"
def process
  step_one()
  step_two()
end
"#;

    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    let mutants = crate::utils::mutants_for_slug(&engine, &target, "ER");

    assert_eq!(
        mutants.len(),
        2,
        "expected one ER mutant per inner statement (not the whole method body), got {mutants:?}"
    );
    assert!(
        mutants.iter().all(|m| m.new_text == "raise \"mewt\""),
        "ER should replace each statement with raise"
    );
}

pub(crate) fn assert_only_slug_and_expected_new_texts(
    source: &str,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let (_tmp, target) = create_test_target(source);
    let engine = RubyLanguageEngine::new();
    utils::assert_only_slug_and_expected_new_texts(&engine, &target, slug, expected_new_texts);
}
