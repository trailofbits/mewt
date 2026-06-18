# Language/Dialect Resolution Contract

[Back to README](../README.md)

## Scope

This contract defines how mewt resolves a target path, optional language label, and config dialect selections into one concrete language engine.

Core owns routing and config validation. Language modules own dialect semantics.

## Key terms

- **Family**: a resolver-owned language family key, such as `move`, `javascript`, or `rust`.
- **Concrete engine**: the exact engine selected for mutation. Dialect-aware families expose one engine per dialect.
- **Canonical label**: the `LanguageEngine::language()` label used for storage, logging, and filtering. Examples: `rust`, `move/sui`, `javascript/tsx`.
- **Dialect**: a language-owned variant string. Core can validate whether a family exposes a dialect key, but only the language resolver interprets it.

## Resolver inputs

A `ResolutionRequest` contains:

- target path
- explicit language label, if supplied
- optional `ResolutionDefaults` from project/per-target config

CLI dialect selection uses dialect-qualified language labels such as `move/iota` or `javascript/tsx`.

## Resolver output

`LanguageResolver::resolve(...)` returns one concrete `&dyn LanguageEngine` or an error.

The returned engine must already represent the selected dialect. After resolution, callers should use `engine.language()` as the target language label. Engines must not infer dialects from `Target.language` during mutation.

## Required resolver behavior

A resolver must provide:

- `family()`: stable config/diagnostic key
- `engines()`: all concrete engines owned by the family
- `dialect_policy()`: dialect keys exposed by this resolver, if any
- `resolve(...)`: target/label/config resolution to a concrete engine
- `filter_labels(...)`: expansion/canonicalization for filter contexts

Filtering is separate from target resolution. A selector such as `move` or `javascript` may expand to all concrete labels for database/listing filters, while target resolution chooses one concrete engine from label, config, extension, or fallback rules.

## Dialect policy

`DialectPolicy` lives in core resolver plumbing because core needs to validate config and diagnostics without importing language-specific dialect config structs.

A non-empty policy means the family accepts dialect selections in config:

```toml
[languages.move]
dialect = "aptos"

[[per_target]]
glob = "legacy-jsx/**/*.js"
languages.javascript.dialect = "jsx"
```

Core validates that the family exists, exposes dialects, and contains the configured key. The resolver still owns final interpretation.

## Precedence policy

Generic precedence:

1. Explicit concrete language label, e.g. `move/iota` or `javascript/tsx`
2. Per-target config dialect, e.g. `[[per_target]].languages.move.dialect`
3. Project config dialect, e.g. `[languages.move].dialect`
4. Extension-derived resolver selection
5. Resolver-owned deterministic fallback, if any

Language-specific interpretation happens inside the resolver.

Move currently resolves `.move` dialects as:

1. concrete label, e.g. `move/iota`
2. per-target `languages.move.dialect`
3. project `[languages.move].dialect`
4. default `sui`

JavaScript resolves dialects as:

1. concrete label, e.g. `javascript/tsx`
2. per-target `languages.javascript.dialect`
3. project `[languages.javascript].dialect`
4. extension: `.js`, `.jsx`, `.ts`, `.tsx`

## Canonical labels and persistence

New targets should be stored using resolver-produced canonical labels, such as:

- `rust`
- `move/sui`
- `move/iota`
- `move/aptos`
- `javascript/js`
- `javascript/jsx`
- `javascript/ts`
- `javascript/tsx`

The store does not own language-specific normalization. Filtering uses registry/resolver expansion and includes the raw query so legacy labels remain matchable where possible.

## Core boundaries

Core may:

- carry generic dialect strings through `ResolutionDefaults`
- validate configured dialect keys against `DialectPolicy`
- store `engine.language()`
- ask the registry for filter label expansion

Core must not:

- import language-specific dialect config structs such as `MoveDialectConfig`
- choose parser/syntax/mutation catalogs for a dialect-aware language
- infer JavaScript dialects beyond asking the JavaScript resolver
- normalize stored labels with language-specific helper imports
