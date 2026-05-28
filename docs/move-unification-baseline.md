# Move Unification Baseline & Safety Net

This note captures the baseline behavior and compatibility touchpoints after the Move unification rollout (Phases 1–6).

## Naming touchpoints

- Canonical user-facing language is `Move`.
- Canonical selectors for Move are:
  - `move`
  - `move/sui`
  - `move/iota`
  - `move/aptos`
- Internal profiled labels for `.move` targets are:
  - `Move/sui`
  - `Move/iota`
  - `Move/aptos`

## Dialect resolution baseline

For `.move` targets, dialect resolves in this order:

1. CLI `--dialect`
2. Config `[languages.move].dialect`
3. Default `sui`

omitting dialect uses defaulting behavior (sui).

## Persistence compatibility baseline

- No schema changes are required.
- Canonical persisted language values are `Move/sui`, `Move/iota`, and `Move/aptos`.
- Legacy persisted `SuiMove` labels are not migrated automatically; purge or regenerate existing databases when upgrading.
- Query/filter logic targets canonical/profiled Move language variants.

## Regression safety net currently in place

- Dialect resolution tests (CLI/config/default and invalid value handling).
- Target loading tests proving `.move` files receive deterministic profiled labels.
- Registry tests proving canonical/profiler Move name resolution.
- Store tests proving canonical Move labels remain queryable by canonical selectors.
- Dialect-aware Move test suites (`Move/sui`, `Move/iota`, and `Move/aptos`).
- Move grammar drift guard tests for critical node/field contracts used by mutation operators.

## Command-level validation baseline

Latest validation for the current state passes:

- `cargo check`
- `just test`
- `just pre-commit`
