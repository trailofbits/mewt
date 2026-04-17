# Mutation Test Suite Conventions

This directory follows a repeatable layout for mutation tests across languages.
The goal is to keep coverage focused, deterministic, and easy to extend.

## Directory Layout

At a glance:

- `tests/utils.rs` contains shared, language-agnostic test helpers used by mutation tests.
- `tests/conformance.rs` contains shared integration-test conformance checks and example-file loading helpers.
- `tests/languages.rs` is the integration-test entry point that wires all language suites and enforces slug/test-module parity.
- `tests/<language>/` contains each language-specific suite.
- `tests/<language>/mod.rs` wires that suite’s submodules.
- `tests/<language>/mutations/` contains one Rust module per mutation slug (for example, `<SLUG>.rs`).
- Keep canonical example fixtures at `tests/<language>/example.<ext>` and exercise them in each language's `integration_tests.rs`.
  - JavaScript is the exception: keep one canonical fixture per supported extension (for example `example.js`, `example.ts`, `example.jsx`, `example.tsx`).

Some language suites also include broader behavior tests (for example integration, parser-focused, or comment-handling tests) alongside `mutations/`.

## Shared Helper Architecture (`tests/utils.rs`)

Use shared helpers first; avoid reimplementing fixture plumbing per language.

Common helpers currently include:

- Target fixture creation by extension or filename:
  - `target_fixture_for_extension(...)`
  - `target_fixture_for_filename(...)`
- Mutant utilities:
  - `mutants_for_slug(...)`
- Shared assertion primitive:
  - `assert_only_slug_and_expected_new_texts(...)`

### Recommended usage pattern

- In each language integration module, keep a small wrapper API (for example `create_test_target(...)`) that delegates to `tests/utils`.
- Keep wrappers only for language-specific concerns (engine construction, default extension, filename selection).
- Run the shared integration conformance harness from `tests/conformance.rs` before adding language-specific integration assertions.
- Call shared assertion/mutant helpers from wrappers rather than duplicating logic in each suite.

## Per-Language Module Structure

Each `tests/<language>/mod.rs` should expose a consistent module mix:

- `mutations/` for per-slug targeted assertions.
- Optional supporting suites (integration, parser behavior, comment handling, regressions, etc.).

Whenever you add a new suite file, add it to the corresponding `mod.rs` so it compiles and runs with the rest of that language’s tests.

## Contribution Flow

### A) Adding or updating a slug test

1. Generate or inspect mutants for the language engine you are changing.
2. Add or update `tests/<language>/mutations/<SLUG>.rs`.
3. Keep fixtures minimal and deterministic.
4. Use existing language integration wrappers (which should delegate into `tests/utils`).
5. If behavior outside a single slug changes, also update broader suites (integration/parser/comment/regression tests).
6. Run `just test` and `just pre-commit` before submitting.

### B) Adding a new language test suite

1. Create `tests/<language>/` with `mod.rs`, `integration_tests.rs`, `mutations/`, and an `example.<ext>` fixture (or multiple `example.*` fixtures for JavaScript extensions).
2. In `tests/languages.rs`, include the new suite module and keep shared helpers:
   - `#[path = "utils.rs"] mod utils;`
   - `mod <language>;`
3. Implement thin language wrappers in `integration_tests.rs` that delegate to `tests/utils`.
4. Add one `mutations/<SLUG>.rs` module per supported slug.
5. Ensure slug/test parity passes via the guard test in `tests/languages.rs`.
6. Run `just test` and `just pre-commit`.

## Slug Coverage Guardrail

The guard test in `tests/languages.rs` compares:

- the mutation slugs registered by each language engine, and
- the set of files present in `tests/<language>/mutations/`.

The test fails if a slug is missing a module or if a module has no corresponding slug. This keeps implementation and test coverage in lockstep.

If you remove or rename a slug, update both the engine registration and the mutation test module in the same change.

## Why This Structure Works

- **Targeted coverage:** Per-slug modules make gaps visible.
- **Deterministic assertions:** Shared helper primitives keep assertions consistent across suites.
- **Lower maintenance cost:** Common fixture/assertion logic lives in one place.
- **Coverage enforcement:** The slug guard catches missing tests as soon as slugs change.
