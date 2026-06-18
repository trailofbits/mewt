# `Language::to_string()` survey

## Refactored to accept `Language`

- `src/core/runner.rs` — Removed the local `target.language.to_string()` values and now pass `&target.language` to registry severity and mutation lookups.
- `src/core/registry.rs` — `resolve_canonical_language` and `resolve_canonical_for_language_label` now return `Language`, `all_languages` now returns `Vec<Language>`, and mutation/severity lookup APIs now accept `&Language`.
- `src/core/cmds/mutate.rs` — Removed the severity lookup stringification and now pass `&target.language`.
- `src/core/cmds/results.rs` — Removed mutation lookup stringification and now pass `&target.language`.
- `src/core/types/target.rs` — Replaced string-based engine lookup with `get_engine_for_language(&Language)` and removed reparsing of resolved canonical languages.
- `src/core/cmds/status.rs` — `compute_severity_catch_rates` now accepts `&Language`, and campaign-wide severity lookup uses `Vec<Language>` from `all_languages`.
- `src/core/cmds/print/mutants.rs` — Removed mutation lookup stringification and now pass `&target.language`.
- `src/core/cmds/print/mutations.rs` — Uses `Language` values from resolver/all-language APIs for engine lookup; remaining `to_string()` calls are for display labels.

## Remaining string conversions that should stay, or are not worth changing

- `src/languages/javascript/resolver.rs` — Test assertions intentionally check the canonical encoded label selected by the resolver.
- `src/languages/cpp/resolver.rs` — `filter_labels` intentionally returns string labels for filtering and SQL-facing call paths, so changing this would require a broader selector/filter redesign.
- `src/languages/solidity/resolver.rs` — `filter_labels` intentionally returns string labels for filtering and SQL-facing call paths, so changing this would require a broader selector/filter redesign.
- `src/languages/move/resolver.rs` — Test assertions intentionally check concrete Move dialect labels.
- `src/languages/rust/resolver.rs` — `filter_labels` intentionally returns string labels, and its test assertions intentionally check rendered labels.
- `src/languages/go/resolver.rs` — `filter_labels` intentionally returns string labels for filtering and SQL-facing call paths, so changing this would require a broader selector/filter redesign.
- `tests/cpp/integration_tests.rs` — The assertion checks that the engine renders as `C++`.
- `src/core/store.rs` — The SQLite schema stores `language` as text, so this boundary must stringify `Language` before binding the value.
- `src/core/cmds/print/mutations.rs` — The remaining conversions create display text for CLI output, not downstream lookup keys.
- `src/core/types/language.rs` — Unit tests directly verify `Display`/`to_string` output.
