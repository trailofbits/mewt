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

- `move/sui`
- `move/iota`
- `move/aptos`

`move` remains useful for filtering and for explicit language selection, but target resolution stores the selected concrete engine label.

## Concrete Move engines

Move now has one concrete engine per dialect:

- `MoveDialectEngine::new(MoveDialect::Sui)` -> `move/sui`
- `MoveDialectEngine::new(MoveDialect::Iota)` -> `move/iota`
- `MoveDialectEngine::new(MoveDialect::Aptos)` -> `move/aptos`

The shared implementation receives already-selected dialect config, parser/syntax support, and mutation catalog data. Engines do not infer dialects from `Target.language` while mutating.

`MoveLanguageEngine::new()` remains a compatibility wrapper that constructs the Sui engine.

## Resolver-owned dialect policy

`src/languages/move/resolver.rs` owns Move dialect resolution:

1. explicit concrete label, e.g. `move/iota`
2. per-target `languages.move.dialect`
3. project `[languages.move].dialect`
4. default `sui`

Core validates configured dialect strings against the resolver's `DialectPolicy`. The Move resolver owns final interpretation.

For explicit labels:

- `--language move` uses config/default dialect selection.
- `--language move/sui`, `--language move/iota`, and `--language move/aptos` select concrete engines directly.

## Mutation catalog behavior

Each concrete Move engine exposes its effective mutation catalog through `get_mutations()`.

Unsupported inherited mutations are filtered when constructing dialect engines. Callers such as `print mutations` do not apply Move-specific mutation filtering.

## Persistence compatibility baseline

- No schema changes are required.
- New targets should store `move/sui`, `move/iota`, or `move/aptos`.
- The store no longer imports Move-specific normalization helpers.
- Database filters use registry expansion. A raw filter query is also included so historical rows such as `move` can still be matched.
- Legacy `SuiMove`-style labels are not migrated automatically; purge or regenerate old databases if needed.

## Regression safety net currently in place

- Move resolver tests cover CLI/config/default precedence and invalid value handling.
- Target loading tests prove `.move` files receive deterministic concrete labels.
- Registry tests prove canonical Move label resolution and filter expansion.
- Store tests prove canonical Move labels are queryable by family and dialect selectors, while raw legacy filter queries remain possible.
- Dialect-aware Move test suites cover `move/sui`, `move/iota`, and `move/aptos`.
- Move grammar drift guard tests cover critical node/field contracts used by mutation operators.

## Command-level validation baseline

Latest validation for the current state passes:

- `cargo check`
- `cargo test`
- `just pre-commit`
