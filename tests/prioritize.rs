use std::process::Command;

use mewt::LanguageEngine;
use mewt::core::prioritize::{Purpose, plan};
use mewt::languages;
use mewt::types::{Hash, Target};

#[test]
fn cli_consent_threshold_and_help() {
    let temp = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_mewt");
    let invoke = |args: &[&str]| {
        Command::new(bin)
            .current_dir(temp.path())
            .env_remove("TYPESAFE_API_KEY")
            .args(args)
            .output()
            .unwrap()
    };
    for mode in ["mutants", "survivors"] {
        let out = invoke(&["prioritize", mode]);
        assert!(!out.status.success());
        let text = String::from_utf8_lossy(&out.stderr);
        assert!(text.contains("https://typesafe.ai/"), "{text}");
        assert!(text.contains("uploads source"), "{text}");
        assert!(!temp.path().join("mewt.sqlite.priorities.sqlite").exists());
    }
    let blank = Command::new(bin)
        .current_dir(temp.path())
        .env("TYPESAFE_API_KEY", "  ")
        .args(["prioritize", "mutants"])
        .output()
        .unwrap();
    assert!(!blank.status.success());
    assert!(String::from_utf8_lossy(&blank.stderr).contains("uploads source"));
    let out = invoke(&["prioritize", "mutants", "--help"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("TypeSafe"));
    for invalid in ["NaN", "inf", "-0.1", "4.1", "oops"] {
        let out = invoke(&["run", "--priority-threshold", invalid]);
        assert!(!out.status.success(), "{invalid}");
    }
    for valid in ["0", "2.5", "4"] {
        let out = invoke(&["run", "--priority-threshold", valid]);
        assert!(out.status.success(), "{valid}: {:?}", out);
    }
    assert!(!temp.path().join("mewt.sqlite.priorities.sqlite").exists());
    let out = invoke(&["run", "some.rs", "--priority-threshold", "2"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("requires saved mutants"));

    // Saved campaigns still work with no TypeSafe key or cached annotations.
    std::fs::write(
        temp.path().join("example.rs"),
        "fn value() -> bool { true }\n",
    )
    .unwrap();
    let out = invoke(&["mutate", "example.rs"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = invoke(&[
        "run",
        "--priority-threshold",
        "4",
        "--test.cmd",
        "true",
        "--comprehensive",
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!temp.path().join("mewt.sqlite.priorities.sqlite").exists());
    let out = invoke(&["results", "--format", "json", "--all"]);
    assert!(out.status.success());
    let results: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(!results["results"].as_array().unwrap().is_empty());
    assert!(results["results"][0].get("test_goal_priority").is_none());
    let out = invoke(&["results", "--format", "ids"]);
    assert!(out.status.success());
}

#[test]
fn source_windows_work_on_all_supported_language_examples() {
    let engines: Vec<(&str, &str, Box<dyn LanguageEngine>)> = vec![
        (
            "tests/cpp/example.cpp",
            "cpp",
            Box::new(languages::cpp::engine::CppLanguageEngine::new()),
        ),
        (
            "tests/daml/example.daml",
            "daml",
            Box::new(languages::daml::engine::DamlLanguageEngine::new()),
        ),
        (
            "tests/go/example.go",
            "go",
            Box::new(languages::go::engine::GoLanguageEngine::new()),
        ),
        (
            "tests/javascript/example.js",
            "javascript/js",
            Box::new(languages::javascript::engine::JavaScriptLanguageEngine::new()),
        ),
        (
            "tests/rust/example.rs",
            "rust",
            Box::new(languages::rust::engine::RustLanguageEngine::new()),
        ),
        (
            "tests/solidity/example.sol",
            "solidity",
            Box::new(languages::solidity::engine::SolidityLanguageEngine::new()),
        ),
        (
            "tests/move/example.move",
            "move/sui",
            Box::new(languages::r#move::engine::MoveLanguageEngine::new()),
        ),
    ];
    for (path, language, engine) in engines {
        let text = std::fs::read_to_string(path).unwrap();
        let target = Target {
            id: 1,
            path: path.into(),
            file_hash: Hash::digest(text.clone()),
            text,
            language: language.parse().unwrap(),
        };
        let mutants = engine
            .mutate(&target)
            .into_iter()
            .enumerate()
            .map(|(index, mut m)| {
                m.id = index as i64 + 1;
                m
            })
            .collect::<Vec<_>>();
        assert!(!mutants.is_empty(), "{path} should produce mutants");
        let planned = plan(&target, mutants.clone(), Purpose::Pre);
        assert!(
            planned.unsupported.is_empty(),
            "{path}: {:?}",
            planned.unsupported
        );
        assert_eq!(
            planned
                .batches
                .iter()
                .map(|b| b.mutants.len())
                .sum::<usize>(),
            mutants.len(),
            "{path}"
        );
    }
}
