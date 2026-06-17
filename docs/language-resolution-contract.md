# Language/Dialect Resolution Contract

[Back to README](../README.md)

## Scope

This contract defines how mewt resolves a target path, explicit language label, optional dialect override, and config defaults into one concrete language engine.

Core owns routing. Language modules own dialect semantics.

## Key terms

- **Family**: a resolver-owned language family key, such as `move`, `javascript`, or `rust`.
- **Concrete engine**: the exact engine selected for mutation. Dialect-aware families expose one engine per dialect.
- **Canonical label**: `LanguageEngine::canonical_name()`. This is the storage, logging, and filtering currency. Examples: `Rust`, `move/sui`, `javascript/tsx`.
- **Dialect**: a language-owned variant string. Core may carry this string through config/CLI plumbing, but only the language resolver interprets it.

## Resolver inputs

A `ResolutionRequest` contains:

- target path
- explicit language label, if supplied
- explicit dialect string, if supplied
- optional `ResolutionDefaults` from config/CLI merging

Core does not parse family-specific dialect values such as `sui`, `iota`, `aptos`, `jsx`, or `tsx`.

## Resolver output

`LanguageResolver::resolve(...)` returns one concrete `&dyn LanguageEngine` or an error.

The returned engine must already represent the selected dialect. After resolution, callers should use `engine.canonical_name()` as the target language label. Engines must not infer dialects from `Target.language` during mutation.

## Required resolver behavior

A resolver must provide:

- `family()`: stable config/diagnostic key
- `engines()`: all concrete engines owned by the family
- `accepts_cli_dialect()`: whether global `--dialect` may be routed to this resolver
- `resolve(...)`: target/label/default resolution to a concrete engine
- `filter_labels(...)`: expansion/canonicalization for filter contexts

Filtering is separate from target resolution. A selector such as `move` may expand to all concrete Move labels for database filtering, while target resolution may choose one concrete default dialect.

## Precedence policy

Generic core precedence:

1. Explicit CLI language label
2. Explicit CLI dialect, routed only to a resolver that accepts CLI dialects
3. Generic config defaults under `[languages.<family>]`
4. Extension-derived resolver selection
5. Resolver-owned deterministic fallback, if any

Language-specific interpretation happens inside the resolver.

Move currently resolves `.move` dialects as:

1. `--dialect`
2. `[languages.move].dialect`
3. default `sui`

JavaScript does not accept CLI/config dialect selection. The `.js`, `.jsx`, `.ts`, and `.tsx` extensions, or explicit labels such as `javascript/tsx`, select concrete JavaScript engines.

## CLI dialect routing

The global `--dialect` flag is generic plumbing, not a universal feature.

- If no registered resolver accepts CLI dialects, core rejects `--dialect`.
- If exactly one resolver accepts CLI dialects, core routes the CLI dialect into that resolver family’s defaults.
- If multiple resolvers accept CLI dialects, core rejects the request as ambiguous unless the call site provides a family-specific language label.

Today, Move accepts CLI dialects; JavaScript does not.

## Canonical labels and persistence

New targets should be stored using resolver-produced canonical labels, such as:

- `Rust`
- `move/sui`
- `move/iota`
- `move/aptos`
- `javascript/js`
- `javascript/jsx`
- `javascript/ts`
- `javascript/tsx`

The store does not own language-specific normalization. Filtering uses registry/resolver expansion and includes the raw query so legacy labels remain matchable where possible.

## Compatibility expectations

Move compatibility:

- `move` remains a family selector.
- `move/sui`, `move/iota`, and `move/aptos` remain concrete selectors.
- Historical stored `move` rows can still be matched by raw filter query inclusion.

JavaScript compatibility:

- `javascript` and `js` filter selectors expand to all concrete JavaScript labels.
- Explicit concrete selectors such as `javascript/ts` filter to one label.
- Project-wide JavaScript dialect config is intentionally unsupported.

## Core boundaries

Core may:

- carry generic dialect strings through `ResolutionDefaults`
- decide which resolver receives a CLI dialect
- store `engine.canonical_name()`
- ask the registry for filter label expansion

Core must not:

- validate Move dialect names
- infer JavaScript dialects beyond asking the JavaScript resolver
- normalize stored labels with language-specific helper imports
- choose parser/syntax/mutation catalogs for a dialect-aware language
