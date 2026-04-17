#[path = "common/mod.rs"]
mod common;

use mewt::LanguageEngine;
use mewt::languages::rust::engine::RustLanguageEngine;

#[test]
fn common_helpers_smoke_test() {
    let fixture =
        common::target_fixture_for_extension("Rust", "rs", "fn demo() { let x = 1 + 2; }");
    let engine = RustLanguageEngine::new();
    let mutants = engine.mutate(fixture.target());

    let _ = common::slug_set(&mutants);
    let _ = common::slug_counts(&mutants);
}
