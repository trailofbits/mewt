# Move Unification Baseline & Safety Net

This note captures the baseline behavior and compatibility touchpoints after the Move unification rollout (Phases 1–6).

## Naming touchpoints

- Canonical user-facing language is `Move`.
- Backward-compatible aliases are accepted in CLI/registry/filtering:
  - `move`
  - `suimove` / `SuiMove`
  - `sui_move`
- Internal profiled labels for `.move` targets are:
  - `Move/sui`
  - `Move/iota`

## Dialect resolution baseline

For `.move` targets, dialect resolves in this order:

1. CLI `--dialect`
2. Config `[languages.move].dialect`
3. Default `sui`

`auto` is treated as defaulting behavior.

## Persistence compatibility baseline

- No schema changes are required.
- Legacy persisted language values (such as `SuiMove`) are normalized on read paths.
- Query/filter logic matches both legacy and profiled/canonical Move language variants.

## Regression safety net currently in place

- Dialect resolution tests (CLI/config/default and invalid value handling).
- Target loading tests proving `.move` files receive deterministic profiled labels.
- Registry alias tests proving canonical + legacy name resolution.
- Store compatibility tests proving legacy `SuiMove` data remains usable.
- Dialect-aware Move test suites (`Move/sui` and `Move/iota`).
- Move grammar drift guard tests for critical node/field contracts used by mutation operators.

## Command-level validation baseline

Latest validation for the current state passes:

- `cargo check`
- `just test`
- `just pre-commit`
