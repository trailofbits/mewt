# Mutation Test Suite Conventions

This directory follows a repeatable layout for mutation tests across languages.
The goal is to keep coverage focused, deterministic, and easy to extend.

## Directory Layout

At a glance:

- `tests/<language>/` contains each language-specific suite.
- `tests/<language>/mod.rs` wires that suite’s submodules.
- `tests/<language>/mutations/` contains one Rust module per mutation slug (for example, `<SLUG>.rs`).
- `tests/<language>/examples/` stores fixture sources used by tests.
- `tests/slug_module_guard.rs` enforces slug/test-module parity.

Some language suites also include broader behavior tests (for example integration or parser-focused tests) alongside `mutations/`.

## Helper Utilities

Test helpers should be reused rather than reimplemented ad hoc.

Current pattern in this repository:

- Helpers are typically defined in the language suite’s integration module (for example target construction and slug filtering helpers).
- Mutation test modules import those helpers from the same language suite.

If shared helpers are introduced later, keep them centralized and generic so all suites can use the same fixture and assertion primitives.

## Per-Language Module Structure

Each `tests/<language>/mod.rs` should expose a consistent module mix:

- `mutations/` for per-slug targeted assertions.
- Optional supporting suites (integration, parser behavior, comment handling, regressions, etc.).

Whenever you add a new suite file, add it to the corresponding `mod.rs` so it compiles and runs with the rest of that language’s tests.

## Adding or Updating a Slug Test

1. Generate or inspect mutants for the language engine you are changing.
2. Add or update `tests/<language>/mutations/<SLUG>.rs`.
3. Keep fixtures minimal and deterministic.
4. Prefer existing helpers for target setup and slug filtering.
5. If behavior outside a single slug changes, also update broader suites (for example integration/parser tests).
6. Run `just test` and `just pre-commit` before submitting.

## Slug Coverage Guardrail

`tests/slug_module_guard.rs` compares:

- the mutation slugs registered by each language engine, and
- the set of files present in `tests/<language>/mutations/`.

The test fails if a slug is missing a module or if a module has no corresponding slug. This keeps implementation and test coverage in lockstep.

If you remove or rename a slug, update both the engine registration and the mutation test module in the same change.

## Why This Structure Works

- **Targeted coverage:** Per-slug modules make gaps visible.
- **Deterministic assertions:** Reused helper patterns reduce flaky or inconsistent checks.
- **Coverage enforcement:** The slug guard catches missing tests as soon as slugs change.
