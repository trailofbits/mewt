# Move Language Unification Plan

## Goal
Transition from a Sui-specific language identity (`SuiMove`) to a dialect-aware Move family model:

- Canonical language: **`Move`**
- Dialects/profiles: **`sui`**, **`iota`**
- Backward compatibility for existing `SuiMove` usage
- Deterministic `.move` dialect resolution via explicit config/CLI first, then fallback strategy

---

## Non-Goals
- Do **not** change SQL schema/migrations unless explicitly approved.
- Do **not** remove compatibility aliases in initial rollout.

---

## Invariants to Preserve
1. Existing Sui Move behavior remains correct during migration.
2. Existing users invoking `--language SuiMove` do not break.
3. Existing stored targets/results remain readable.
4. CI/pre-commit checks remain green after every phase.

---

## Phase 0 — Baseline & Safety Net

### Objective
Capture current behavior and lock in regression coverage before refactor.

### Tasks
- [x] Document current Move behavior and naming touchpoints (CLI, registry, tests, docs, persistence).
- [x] Add/confirm regression tests for current `SuiMove` flows.
- [x] Add targeted tests for `.move` target loading and language selection behavior.

### Completion Criteria
- [x] A short baseline summary is committed under `docs/` or in test comments.
- [x] Current Move-related tests pass locally.
- [x] `just pre-commit` passes.

---

## Phase 1 — Introduce Canonical `Move` Name (No Behavior Change)

### Objective
Establish `Move` as canonical while preserving old names.

### Tasks
- [x] Rename user-facing identity from `SuiMove` to `Move` where safe.
- [x] Add aliases so language lookup accepts: `move`, `SuiMove`, `sui_move`.
- [x] Keep existing parser/mutation behavior identical (still Sui grammar/profile internally for now).
- [x] Update CLI/help text and docs to mention canonical `Move` + compatibility aliases.

### Completion Criteria
- [x] `mewt print mutations --language move` works.
- [x] `mewt print mutations --language suimove` still works (alias).
- [x] Existing tests pass plus alias tests pass.
- [x] `just pre-commit` passes.

---

## Phase 2 — Add Dialect Concept at API/Config Layer

### Objective
Introduce a first-class dialect setting without forcing schema changes.

### Tasks
- [x] Add dialect option to CLI (`--dialect`) for Move commands where language selection matters.
- [x] Add config support in `mewt.toml` (e.g. `[languages.move] dialect = "sui|iota|auto"`).
- [x] Define and document dialect resolution order:
  1) CLI `--dialect`
  2) config dialect
  3) fallback default (`sui`) with clear warning when ambiguity exists.
- [x] Keep behavior unchanged when dialect is omitted (defaults to current Sui behavior).

### Completion Criteria
- [x] Explicit dialect selection is wired and test-covered.
- [x] Omitted dialect preserves prior behavior.
- [x] Resolution-order tests exist and pass.
- [x] `just pre-commit` passes.

---

## Phase 3 — Move Engine Refactor to Dialect Profiles

### Objective
Implement a single Move family engine with dialect profiles.

### Tasks
- [x] Create/introduce a unified `move` language module/engine.
- [x] Extract dialect profile boundary (syntax constants, parser handle, feature flags).
- [x] Implement at least `sui` and `iota` profiles behind the same engine interface.
- [x] Ensure `.move` loading uses resolved dialect (from Phase 2), not extension-only inference.

### Completion Criteria
- [x] One canonical Move engine serves both `sui` and `iota` profiles.
- [x] `.move` files mutate under selected dialect deterministically.
- [x] No extension-collision ambiguity in registry behavior.
- [x] `just pre-commit` passes.

---

## Phase 4 — Backward Compatibility for Persisted Data

### Objective
Ensure old stored language identities still work.

### Tasks
- [x] Add read-path compatibility mapping for legacy stored `SuiMove` targets/results.
- [x] Decide migration strategy:
  - **Preferred initially:** lazy/in-memory mapping (no schema change).
  - **Optional later:** explicit DB migration (requires user approval).
- [x] Add tests proving old records remain usable.

### Completion Criteria
- [x] Running commands on legacy data does not fail due to missing engine.
- [x] Compatibility tests for legacy `SuiMove` entries pass.
- [x] If schema migration is proposed, approval is recorded before implementation.
- [x] `just pre-commit` passes.

---

## Phase 5 — Test Matrix Expansion (Dialect-Aware)

### Objective
Ensure correctness across dialects and prevent drift.

### Tasks
- [x] Reorganize/add tests under a dialect-aware structure (e.g. `tests/move/sui`, `tests/move/iota`).
- [x] Keep shared conformance tests plus dialect-specific expectations.
- [x] Add parser/grammar drift guard tests for critical node/field dependencies used by mutation operators.

### Completion Criteria
- [x] Both `sui` and `iota` dialect test suites run and pass.
- [x] Shared and dialect-specific mutation expectations are explicit.
- [x] Drift guard tests exist and fail clearly when grammar contracts break.
- [x] `just pre-commit` passes.

---

## Phase 6 — Documentation & UX Finalization

### Objective
Make the new model obvious and easy to use.

### Tasks
- [x] Update `README.md` supported languages to `Move` with dialect notes.
- [x] Add a short "Choosing Move dialect" section with examples.
- [x] Update any docs/examples that still imply Sui-only naming.
- [x] Add deprecation notice timeline for `SuiMove` alias removal (if desired).

### Completion Criteria
- [x] Docs consistently use canonical `Move` terminology.
- [x] CLI examples include dialect usage.
- [x] No stale `SuiMove` wording remains except intentional compatibility docs.
- [x] `just pre-commit` passes.

---

## Definition of Done (End State)

All items below must be true:
- [x] Canonical language identity is `Move`.
- [x] `sui` and `iota` are selectable dialects in config/CLI.
- [x] Legacy `SuiMove` usage still works via aliases/compat layer.
- [x] `.move` dialect resolution is deterministic and documented.
- [x] Tests are dialect-aware and passing.
- [x] Documentation reflects the Move family model.
- [x] `just pre-commit` passes on final branch.

---

## Execution Notes for Future Workers

- Keep changes phase-scoped; avoid mixing refactor + feature + doc cleanup in one large PR.
- After each phase, run:
  - `just pre-commit`
  - `just test` (if phase touches behavior)
- If a schema change seems necessary, stop and request explicit approval before editing `migrations/`.
- Do not remove aliases (`SuiMove`, `sui_move`) until a later, explicitly approved deprecation phase.
