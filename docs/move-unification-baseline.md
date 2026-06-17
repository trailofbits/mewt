# Move Unification Baseline & Safety Net

This note captures current Move behavior after the dialect-resolution refactor.

[Back to README](../README.md)

## Naming touchpoints

Canonical selectors for Move are:

- `move` for the Move family
- `move/sui`
- `move/iota`
- `move/aptos`

Canonical persisted target labels are concrete dialect labels:

- `Move/sui`
- `Move/iota`
- `Move/aptos`

`move` remains useful for filtering and for explicit language selection, but target resolution stores the selected concrete engine label.

## Concrete Move engines

Move now has one concrete engine per dialect:

- `MoveDialectEngine::new(MoveDialect::Sui)` -> `Move/sui`
- `MoveDialectEngine::new(MoveDialect::Iota)` -> `Move/iota`
- `MoveDialectEngine::new(MoveDialect::Aptos)` -> `Move/aptos`

The shared implementation receives already-selected dialect config, parser/syntax support, and mutation catalog data. Engines do not infer dialects from `Target.language` while mutating.

`MoveLanguageEngine::new()` remains a compatibility wrapper that constructs the Sui engine.

## Resolver-owned dialect policy

`src/languages/move/resolver.rs` owns Move dialect policy:

1. CLI `--dialect`
2. Config `[languages.move].dialect`
3. Default `sui`

The resolver validates configured and CLI dialect strings. Core config stores generic dialect strings and does not know the valid Move values.

For explicit labels:

- `--language move` uses CLI/config/default dialect selection.
- `--language move/sui`, `--language move/iota`, and `--language move/aptos` select concrete engines directly.
- Combining a concrete Move label with `--dialect` is rejected.

## Mutation catalog behavior

Each concrete Move engine exposes its effective mutation catalog through `get_mutations()`.

Unsupported inherited mutations are filtered when constructing dialect engines. Callers such as `print mutations` do not apply Move-specific mutation filtering.

## Persistence compatibility baseline

- No schema changes are required.
- New targets should store `Move/sui`, `Move/iota`, or `Move/aptos`.
- The store no longer imports Move-specific normalization helpers.
- Database filters use registry expansion. A raw filter query is also included so historical rows such as `move` can still be matched.
- Legacy `SuiMove`-style labels are not migrated automatically; purge or regenerate old databases if needed.

## Regression safety net currently in place

- Move resolver tests cover CLI/config/default precedence and invalid value handling.
- Target loading tests prove `.move` files receive deterministic concrete labels.
- Registry tests prove canonical Move label resolution and filter expansion.
- Store tests prove canonical Move labels are queryable by family and dialect selectors, while raw legacy filter queries remain possible.
- Dialect-aware Move test suites cover `Move/sui`, `Move/iota`, and `Move/aptos`.
- Move grammar drift guard tests cover critical node/field contracts used by mutation operators.

## Command-level validation baseline

Latest validation for the current state passes:

- `cargo check`
- `cargo test`
- `just pre-commit`
