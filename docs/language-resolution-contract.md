# Language/Dialect Resolution Contract (Phase 1)

[Back to README](../README.md)

## Scope

This contract defines how mewt resolves an effective language selection for a target path, including dialect-aware languages like Move.

## Resolver inputs

Required inputs:
- Target path (or extension candidate set)
- Explicit language override (optional)
- Explicit dialect override (optional)
- Loaded config defaults
- Registry language metadata (canonical keys, supported extensions, dialect support)

## Normalized selection type

Resolver output should be one normalized value used everywhere:

- `language_key`: canonical base language key (example: `Move`)
- `dialect`: optional dialect (example: `sui`, `iota`; `None` for non-dialect languages)
- `canonical_label`: storage/log label (examples: `Rust`, `Move/sui`, `Move/iota`)
- `source`: where resolution came from (CLI, config, extension/fallback)
- `defaulted`: whether fallback/default behavior was used

## Precedence policy

Resolution precedence (highest to lowest):
1. Explicit CLI language override
2. Explicit CLI dialect override
3. Config language default
4. Config dialect default
5. Extension-derived candidate(s)
6. Deterministic fallback rule for unresolved ambiguity

Move-specific policy (current behavior baseline):
- Dialect precedence: `--dialect` > `[languages.move].dialect` > default `sui`
- if omitted, dialect defaults to `sui` and is marked `defaulted=true`

## Ambiguity policy

- If an extension maps to one language, select it directly.
- If an extension maps to multiple languages, resolver must either:
  - disambiguate with explicit override/config, or
  - apply deterministic fallback and mark `defaulted=true`, or
  - fail with a clear actionable error if no safe fallback exists.
- Canonical labels must be stable and deterministic for persistence and filtering.

## Current call-site inventory to migrate

1. `src/core/main_shared.rs`
   - Resolves Move dialect per command before dispatch (`config().resolve_move_dialect(...)`).
2. `src/core/types/target.rs`
   - `resolve_language_for_path(...)` special-cases `.move` and uses `language_name_for_dialect(...)`.
3. `src/core/registry.rs`
   - `get_engine(...)` includes Move alias matching logic.
   - `language_from_path(...)` returns first extension match (no ambiguity contract).
4. `src/core/cmds/run.rs`
   - Move-specific post-resolution warnings and checks via `is_move_language_name(...)`.
5. `src/core/cmds/mutate.rs`
   - Same Move-specific checks/warnings as run.
6. `src/core/cmds/print/mutations.rs`
   - Local duplicated Move-name detection helper and dialect-resolution behavior.
7. `src/core/store.rs`
   - Read/filter normalization helpers (`normalize_stored_target_language`, `language_filter_variants`) carry alias logic.
8. `src/languages/move/dialect.rs`
   - Canonicalization utilities currently consumed from multiple layers.

## Migration target for Phase 2+

- Move all normalization and ambiguity decisions into a resolver API owned by registry/resolver layer.
- Make commands/store/target loading consume the normalized selection instead of re-implementing alias/dialect logic.
