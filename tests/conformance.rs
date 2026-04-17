use std::collections::HashSet;
use std::path::Path;

use mewt::LanguageEngine;
use mewt::types::Target;
use tempfile::TempDir;

pub struct CommonConformanceSources<'a> {
    pub basic_source: &'a str,
    pub comment_source: &'a str,
    pub complex_source: &'a str,
    pub line_coverage_source: &'a str,
}

pub struct CommonConformanceExpectations<'a> {
    pub language_name: &'a str,
    pub min_complex_mutants: usize,
}

pub fn run_common_language_checks<CreateTarget, MakeEngine>(
    create_target: CreateTarget,
    make_engine: MakeEngine,
    sources: CommonConformanceSources<'_>,
    expectations: CommonConformanceExpectations<'_>,
) where
    CreateTarget: Fn(&str) -> (TempDir, Target),
    MakeEngine: Fn() -> Box<dyn LanguageEngine>,
{
    let (_tmp, target) = create_target(sources.basic_source);
    let basic_mutants = make_engine().mutate(&target);
    assert!(
        !basic_mutants.is_empty(),
        "{} should generate at least one mutant for basic source",
        expectations.language_name
    );

    let slugs: HashSet<&str> = basic_mutants
        .iter()
        .map(|m| m.mutation_slug.as_str())
        .collect();
    assert!(
        slugs.len() > 1,
        "{} should generate diverse mutation slugs for basic source",
        expectations.language_name
    );

    let (_tmp, target) = create_target(sources.comment_source);
    let comment_mutants = make_engine().mutate(&target);
    let comment_only_line_mutations = comment_mutants
        .iter()
        .filter(|m| m.old_text.trim_start().starts_with("//"))
        .count();

    assert_eq!(
        comment_only_line_mutations, 0,
        "{} should not mutate comment-only lines",
        expectations.language_name
    );

    let (_tmp, target) = create_target(sources.complex_source);
    let complex_mutants = make_engine().mutate(&target);
    assert!(
        complex_mutants.len() >= expectations.min_complex_mutants,
        "{} complex source should generate at least {} mutants, got {}",
        expectations.language_name,
        expectations.min_complex_mutants,
        complex_mutants.len()
    );

    let (_tmp, target) = create_target(sources.line_coverage_source);
    let coverage_mutants = make_engine().mutate(&target);
    let lines_touched: HashSet<usize> = coverage_mutants
        .iter()
        .map(|m| m.line_offset as usize)
        .collect();
    assert!(
        lines_touched.len() > 1,
        "{} mutations should touch multiple lines for reasonable coverage",
        expectations.language_name
    );
}

pub fn read_example_source(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read example source at {:?}: {}", path, err))
}
