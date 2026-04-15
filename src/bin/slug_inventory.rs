use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use mewt::LanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;
use serde::Serialize;

#[derive(Serialize)]
struct LanguageInventory {
    language: &'static str,
    language_key: &'static str,
    slug_count: usize,
    slugs: Vec<String>,
    covered_slugs: Vec<String>,
    missing_slugs: Vec<String>,
    extra_test_modules: Vec<String>,
    slug_test_modules: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct InventoryReport {
    generated_from: &'static str,
    languages: Vec<LanguageInventory>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let report = InventoryReport {
        generated_from: "src/bin/slug_inventory.rs",
        languages: vec![
            build_inventory(
                "Go",
                "go",
                GoLanguageEngine::new(),
                Path::new("tests/go/mutations"),
            )?,
            build_inventory(
                "JavaScript",
                "javascript",
                JavaScriptLanguageEngine::new(),
                Path::new("tests/javascript/mutations"),
            )?,
            build_inventory(
                "Rust",
                "rust",
                RustLanguageEngine::new(),
                Path::new("tests/rust/mutations"),
            )?,
            build_inventory(
                "Solidity",
                "solidity",
                SolidityLanguageEngine::new(),
                Path::new("tests/solidity/mutations"),
            )?,
        ],
    };

    let json = serde_json::to_string_pretty(&report)?;
    println!("{}", json);

    Ok(())
}

fn build_inventory<E: LanguageEngine>(
    language: &'static str,
    language_key: &'static str,
    engine: E,
    tests_dir: &Path,
) -> Result<LanguageInventory, Box<dyn Error>> {
    let slug_set: BTreeSet<String> = engine
        .get_mutations()
        .iter()
        .map(|mutation| mutation.slug.to_string())
        .collect();

    let slug_count = slug_set.len();
    let slugs: Vec<String> = slug_set.iter().cloned().collect();

    let test_modules = collect_test_modules(tests_dir)?;

    let covered_slugs: Vec<String> = slug_set
        .intersection(&test_modules.normalized)
        .cloned()
        .collect();
    let missing_slugs: Vec<String> = slug_set
        .difference(&test_modules.normalized)
        .cloned()
        .collect();
    let extra_test_modules: Vec<String> = test_modules
        .normalized
        .difference(&slug_set)
        .map(|slug| {
            test_modules
                .stems_by_slug
                .get(slug)
                .cloned()
                .unwrap_or_else(|| slug.to_ascii_lowercase())
        })
        .collect();

    let slug_test_modules: BTreeMap<String, String> = test_modules
        .stems_by_slug
        .iter()
        .map(|(slug, stem)| {
            (
                slug.clone(),
                format!("tests/{}/mutations/{}.rs", language_key, stem),
            )
        })
        .collect();

    Ok(LanguageInventory {
        language,
        language_key,
        slug_count,
        slugs,
        covered_slugs,
        missing_slugs,
        extra_test_modules,
        slug_test_modules,
    })
}

struct TestModules {
    normalized: BTreeSet<String>,
    stems_by_slug: BTreeMap<String, String>,
}

fn collect_test_modules(path: &Path) -> Result<TestModules, Box<dyn Error>> {
    let mut normalized = BTreeSet::new();
    let mut stems_by_slug = BTreeMap::new();

    if !path.exists() {
        return Ok(TestModules {
            normalized,
            stems_by_slug,
        });
    }

    for entry in fs::read_dir(PathBuf::from(path))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        if path.file_stem().and_then(|stem| stem.to_str()) == Some("mod") {
            continue;
        }

        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            let slug = stem.to_ascii_uppercase();
            normalized.insert(slug.clone());
            stems_by_slug.insert(slug, stem.to_string());
        }
    }

    Ok(TestModules {
        normalized,
        stems_by_slug,
    })
}
