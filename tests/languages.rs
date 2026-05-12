#[path = "conformance.rs"]
mod conformance;
#[path = "utils.rs"]
mod utils;

mod cpp;
mod go;
mod javascript;
mod r#move;
mod rust;
mod solidity;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use mewt::LanguageEngine;
use mewt::languages::cpp::engine::CppLanguageEngine;
use mewt::languages::go::engine::GoLanguageEngine;
use mewt::languages::javascript::engine::JavaScriptLanguageEngine;
use mewt::languages::r#move::engine::MoveLanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;
use mewt::languages::solidity::engine::SolidityLanguageEngine;

#[test]
fn every_mutation_slug_has_a_per_language_test_module() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let cpp = CppLanguageEngine::new();
    check_language(manifest_dir, "C++", "cpp", &cpp);

    let rust = RustLanguageEngine::new();
    check_language(manifest_dir, "Rust", "rust", &rust);

    let go = GoLanguageEngine::new();
    check_language(manifest_dir, "Go", "go", &go);

    let javascript = JavaScriptLanguageEngine::new();
    check_language(manifest_dir, "JavaScript", "javascript", &javascript);

    let solidity = SolidityLanguageEngine::new();
    check_language(manifest_dir, "Solidity", "solidity", &solidity);

    let move_language = MoveLanguageEngine::new();
    check_language(manifest_dir, "Move", "move", &move_language);
}

fn check_language(
    manifest_dir: &Path,
    language_name: &str,
    language_dir: &str,
    engine: &dyn LanguageEngine,
) {
    let slug_set: BTreeSet<String> = engine
        .get_mutations()
        .iter()
        .map(|m| m.slug.to_string())
        .collect();

    let modules_dir = manifest_dir
        .join("tests")
        .join(language_dir)
        .join("mutations");
    assert!(
        modules_dir.is_dir(),
        "expected mutation test directory for {language_name} at {modules_dir:?}"
    );

    let file_slugs: BTreeSet<String> = fs::read_dir(&modules_dir)
        .unwrap_or_else(|err| {
            panic!(
                "failed to read mutation test directory for {language_name} at {modules_dir:?}: {err}"
            )
        })
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                return None;
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(|s| s.to_string())?;
            if stem == "mod" {
                return None;
            }
            Some(stem)
        })
        .collect();

    let missing: Vec<String> = slug_set.difference(&file_slugs).cloned().collect();
    let unexpected: Vec<String> = file_slugs.difference(&slug_set).cloned().collect();

    assert!(
        missing.is_empty(),
        "{language_name} is missing mutation test modules for slugs: {missing:?}"
    );
    assert!(
        unexpected.is_empty(),
        "{language_name} has mutation test modules without corresponding slugs: {unexpected:?}"
    );
}
