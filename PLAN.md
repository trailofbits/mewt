# Registry-Centric Language/Dialect Resolution Plan

## Goal
Refactor language selection so it is owned by the registry/resolver layer (not hand-threaded through command and target code), and make it generic enough for any ambiguous-extension language family (Move now, JS/JSX-style families later).

This plan intentionally **drops backward-compatibility guarantees for legacy naming** like `SuiMove`.

---

## Explicit Policy Decisions

1. **Legacy naming can break**
   - We do not preserve `SuiMove`, `sui_move`, `suimove` aliases unless they happen to remain incidentally.
   - Canonical naming is `Move` + dialect.

2. **Prefer no DB migrations**
   - We should avoid schema changes if we can preserve correctness and maintainability.

3. **Migration is allowed if correctness requires it**
   - If we cannot represent dialect identity safely in persistence (especially for target identity/dedup) without hacks, we should introduce a migration.

---

## Core Question to Resolve Early

Should dialect be persisted as a first-class column (e.g., `targets.dialect`) or encoded in language label (e.g., `Move/iota`)?

### Option A (Preferred starting point): No migration
- Keep storing canonical language label (including dialect where needed, e.g. `Move/sui` and `Move/iota`).
- Centralize normalization in one resolver/registry path.
- Pros: no migration risk, faster rollout.
- Cons: target identity and dedup semantics may remain awkward if dialect-sensitive behavior diverges further.

### Option B: Add first-class dialect field via migration
- Add `dialect` column where appropriate (`targets`, maybe related tables), and enforce identity/query semantics explicitly.
- Pros: cleaner model, fewer string-encoding tricks, better long-term correctness.
- Cons: migration complexity and rollout burden.

---

## Target End State

### Functional End State
- Effective selection is computed by one resolver from:
  - explicit language override
  - explicit dialect override
  - config defaults
  - extension candidates
  - deterministic fallback rules
- Commands (`run`, `mutate`, `print mutations`, target loading paths) consume one resolved selection object.
- Ambiguous extensions are handled by the same mechanism across language families.

### Architectural End State
- Registry/resolver owns:
  - canonical language keys
  - dialect validation for dialect-aware languages
  - extension ambiguity handling
  - canonical labels used in logs/store
- No Move-specific branching in target path resolution.
- No duplicated alias/dialect normalization logic scattered across core/store/print.

### Quality End State
- Test suite includes:
  - precedence tests
  - ambiguity resolution tests
  - dialect-divergence tests (Sui-only/IOTA-sensitive constructs)
- `just pre-commit` and `just test` pass.

---

## Phase 0 — Persistence Decision Gate

### Objective
Decide whether no-migration storage is safe enough or whether first-class dialect persistence is required.

### Tasks
- [x] Document current target identity/dedup behavior and how dialect interacts with it (in-chat).
- [x] Produce 2–3 concrete failure scenarios if dialects diverge (same file hash/path under different dialects).
- [x] Evaluate Option A (label-only) vs Option B (dialect column) against those scenarios.
- [x] Record a decision and rationale (in-chat only; no docs update by request).

### Phase 0 Decision (Completed)
- **Decision:** No DB migration for dialect column right now.
- **Rationale:** Dialect is a resolver-level disambiguation mechanism (per-target selectable, with optional project defaults), and persisting canonical `target.language` labels is sufficient for current goals.
- **Follow-up constraint:** Revisit schema only if/when dialect-sensitive identity or cross-campaign dedup semantics require first-class persistence beyond canonical language labels.

### Exit Criteria
- [x] Decision is explicit: **No migration now** or **Migration required now**.
- [x] Decision includes clear correctness rationale, not preference-only.
- [x] If migration is chosen, user approval is obtained before editing `migrations/`.

---

## Phase 1 — Define Resolver Contract

### Objective
Freeze a single API contract for registry-centric resolution.

### Tasks
- [x] Add a short design doc describing resolver inputs/outputs.
- [x] Define a normalized selection type (language, optional dialect, canonical label).
- [x] Define precedence and ambiguity policy unambiguously.
- [x] Inventory all current call sites that resolve language/dialect.

### Exit Criteria
- [x] Resolver contract doc exists and is referenced from docs.
- [x] Selection type and precedence are unambiguous.
- [x] All existing resolution call sites are listed for migration.

---

## Phase 2 — Implement Resolver Core in Registry

### Objective
Build the resolver in/near `LanguageRegistry` and make it the source of truth.

### Tasks
- [x] Implement resolver API (path + explicit overrides + config => resolved selection).
- [x] Centralize normalization helpers used by registry/store/print.
- [x] Add dialect-aware metadata/hooks for Move.
- [x] Add unit tests for precedence, ambiguity, and canonical labeling.

### Exit Criteria
- [x] Resolver tests pass and cover precedence + ambiguity.
- [x] Shared normalization path exists and is used by at least two subsystems.
- [x] `just pre-commit` passes.

---

## Phase 3 — Migrate Commands and Target Resolution

### Objective
Remove hand-threaded dialect handling from command and target flows.

### Tasks
- [ ] Refactor target resolution to use registry resolver (no Move-specific branch logic).
- [ ] Update `run`, `mutate`, `print mutations` to consume resolved selection.
- [ ] Remove duplicated command-level language/dialect branching.
- [ ] Keep effective-config/log output derived from resolver output.

### Exit Criteria
- [ ] No Move-specific branch remains in target language resolution.
- [ ] Commands use shared resolver path for final selection.
- [ ] Integration tests pass.
- [ ] `just pre-commit` passes.

---

## Phase 4 — Divergence-Focused Tests and Grammar Clarity

### Objective
Make differences between Move dialect grammars explicit and test-enforced.

### Tasks
- [ ] Reorganize Move tests to `tests/move/shared`, `tests/move/sui`, `tests/move/iota`.
- [ ] Add grammar-difference tests (at least one Sui-only construct, one IOTA-sensitive construct).
- [ ] Add resolver integration tests showing deterministic dialect selection for `.move`.
- [ ] Ensure unsupported/invalid dialect selections fail clearly.

### Exit Criteria
- [ ] Dialect-difference tests pass and fail meaningfully on regressions.
- [ ] Resolver integration tests for `.move` ambiguity pass.
- [ ] `just test` and `just pre-commit` pass.

---

## Phase 5 — Optional Schema Migration (Only If Chosen in Phase 0)

### Objective
If required by Phase 0 decision, persist dialect as first-class data.

### Tasks
- [ ] Add migration(s) for dialect persistence fields/indexes.
- [ ] Update model/types/store queries accordingly.
- [ ] Add migration/backfill tests.
- [ ] Run `just reset-db` and full test suite.

### Exit Criteria
- [ ] Migration is applied and tested successfully.
- [ ] Persistence semantics for dialect-sensitive identity are explicit.
- [ ] `just test` and `just pre-commit` pass.

---

## Definition of Done

- [ ] Registry/resolver is authoritative for language+dialect resolution.
- [ ] Target/command flows no longer hand-thread Move-specific dialect logic.
- [ ] Legacy `SuiMove` compatibility is not required and not tested.
- [ ] `.move` ambiguity handling is deterministic and test-covered.
- [ ] Divergence between Sui and IOTA grammar behavior is explicit in tests.
- [ ] Persistence strategy is explicitly justified (no migration or migration).
- [ ] `just pre-commit` and `just test` pass.

---

## Execution Notes

- Keep phase scope tight; avoid mixing broad refactors with feature additions.
- After each phase, run `just pre-commit`.
- Run `just test` for any behavior-affecting phase.
- Do not edit `migrations/` unless Phase 0 selected migration and approval is confirmed.
