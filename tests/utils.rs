#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

use mewt::LanguageEngine;
use mewt::types::{Hash, Mutant, Target};
use tempfile::TempDir;

/// Test fixture that owns a temporary source file and the corresponding [`Target`].
#[derive(Debug)]
pub struct TargetFixture {
    temp_dir: TempDir,
    target: Target,
}

impl TargetFixture {
    /// Create a fixture from an explicit filename (for example `test.tsx`).
    pub fn from_filename(language: impl Into<String>, filename: &str, source: &str) -> Self {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir for test target");
        let path = temp_dir.path().join(filename);
        std::fs::write(&path, source).expect("failed to write test source");

        let text = source.to_string();
        let target = Target {
            id: 1,
            path,
            file_hash: Hash::digest(text.clone()),
            text,
            language: language.into(),
        };

        Self { temp_dir, target }
    }

    /// Create a fixture from a file extension (for example `rs` -> `test.rs`).
    pub fn from_extension(language: impl Into<String>, extension: &str, source: &str) -> Self {
        let filename = format!("test.{extension}");
        Self::from_filename(language, &filename, source)
    }

    /// Borrow the underlying [`Target`].
    pub fn target(&self) -> &Target {
        &self.target
    }

    /// Consume the fixture, returning the owned [`Target`].
    pub fn into_target(self) -> Target {
        self.target
    }

    /// Consume the fixture, returning both [`TempDir`] and [`Target`].
    pub fn into_parts(self) -> (TempDir, Target) {
        (self.temp_dir, self.target)
    }

    /// Borrow the [`TempDir`] keeping the source file alive.
    pub fn temp_dir(&self) -> &TempDir {
        &self.temp_dir
    }

    /// Return the on-disk path for the target file.
    pub fn path(&self) -> &Path {
        self.target.path.as_path()
    }

    /// Borrow the original source text.
    pub fn text(&self) -> &str {
        &self.target.text
    }
}

/// Build a target fixture using an explicit filename.
pub fn target_fixture_for_filename(
    language: impl Into<String>,
    filename: &str,
    source: &str,
) -> TargetFixture {
    TargetFixture::from_filename(language, filename, source)
}

/// Build a target fixture using a file extension.
pub fn target_fixture_for_extension(
    language: impl Into<String>,
    extension: &str,
    source: &str,
) -> TargetFixture {
    TargetFixture::from_extension(language, extension, source)
}

/// Collect mutants produced for a single mutation slug.
pub fn mutants_for_slug(engine: &dyn LanguageEngine, target: &Target, slug: &str) -> Vec<Mutant> {
    engine
        .mutate(target)
        .into_iter()
        .filter(|m| m.mutation_slug == slug)
        .collect()
}

/// Assert that mutants for a given slug only produce expected replacement snippets.
pub fn assert_only_slug_and_expected_new_texts(
    engine: &dyn LanguageEngine,
    target: &Target,
    slug: &str,
    expected_new_texts: &[&str],
) {
    let mutants = engine.mutate(target);

    let selected: Vec<_> = mutants.iter().filter(|m| m.mutation_slug == slug).collect();
    assert!(!selected.is_empty(), "expected at least one {slug} mutant");

    let normalize = |text: &str| text.trim().replace('\r', "");

    let mut covered_tokens: HashSet<&str> = HashSet::new();
    let mut unexpected_mutants: Vec<String> = Vec::new();

    for mutant in &selected {
        let matches: Vec<&str> = expected_new_texts
            .iter()
            .copied()
            .filter(|needle| mutant.new_text.contains(needle))
            .collect();

        if matches.is_empty() {
            unexpected_mutants.push(normalize(&mutant.new_text));
        } else {
            for needle in matches {
                covered_tokens.insert(needle);
            }
        }
    }

    assert!(
        unexpected_mutants.is_empty(),
        "found {slug} mutants with unexpected replacements: {unexpected_mutants:?}"
    );

    for expected in expected_new_texts {
        assert!(
            covered_tokens.contains(expected),
            "missing expected {slug} mutant containing: {expected}"
        );
    }
}

/// Count how many mutants share each slug.
pub fn slug_counts(mutants: &[Mutant]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for mutant in mutants {
        *counts.entry(mutant.mutation_slug.clone()).or_default() += 1;
    }
    counts
}

/// Collect distinct slugs present in the mutant set.
pub fn slug_set(mutants: &[Mutant]) -> HashSet<String> {
    mutants.iter().map(|m| m.mutation_slug.clone()).collect()
}

/// Return the first mutant (ordered by byte offset) for a slug.
pub fn first_mutant_with_slug<'a>(mutants: &'a [Mutant], slug: &str) -> Option<&'a Mutant> {
    mutants
        .iter()
        .filter(|m| m.mutation_slug == slug)
        .min_by_key(|m| m.byte_offset)
}

/// Sort mutants in place by byte offset.
pub fn sort_by_byte_offset(mutants: &mut [Mutant]) {
    mutants.sort_by_key(|m| m.byte_offset);
}
