# Dialect resolution and per-dialect engine plan

## Purpose

This document records the current design direction for making dialect support generic without making JavaScript or Move special in `src/core`.

The new direction is:

> A resolver represents a language family. A concrete engine represents exactly one resolved language/dialect. The resolver performs all dialect resolution and chooses the concrete engine. Core stores and routes by the engine's canonical name.

This keeps core language-agnostic, removes Move-specific config semantics from core, keeps JavaScript extension-driven, and gives us a clean place for dialect-specific mutation catalogs such as Sui-only or TSX-only mutations.

## Status as of 2026-05-28

Substantial implementation is complete:

- The resolver trait now represents language families and returns concrete engines.
- The registry now stores/routes by concrete engine canonical labels.
- Move has concrete Sui, IOTA, and Aptos engines.
- JavaScript has concrete JS, JSX, TS, and TSX engines.
- Engines no longer infer dialects from `Target.language` in their mutation paths.
- Effective mutation catalogs now live on concrete engines; Move filters unsupported inherited mutations at engine construction time.
- Core config now uses generic `[languages.<family>].dialect` storage.
- Move dialect parsing, aliases, defaults, grammar selection, and config validation live under `src/languages/move`.
- JavaScript remains extension-driven and does not accept CLI/config dialect overrides.
- CLI `--dialect` routing is generic through `LanguageRegistry::cli_dialect_family()`.
- Core print/config/target plumbing has been generalized.
- Store-side Move/JavaScript label normalization imports were removed; store filtering now uses registry expansion and includes the raw query to keep legacy stored labels filterable.

The plan below retains the original design rationale, but the priority has shifted from architecture refactor to cleanup, documentation, and regression tests.

The most important invariant is the one-to-one relationship:

```text
one concrete canonical label <-> one concrete engine <-> one dialect configuration
```

Examples:

```text
Move/sui        <-> MoveSuiEngine        <-> Sui grammar/syntax/mutations
Move/iota       <-> MoveIotaEngine       <-> IOTA grammar/syntax/mutations
Move/aptos      <-> MoveAptosEngine      <-> Aptos grammar/syntax/mutations
JavaScript/js   <-> JavaScriptJsEngine   <-> JS grammar/mutations
JavaScript/jsx  <-> JavaScriptJsxEngine  <-> JSX grammar/mutations
JavaScript/ts   <-> JavaScriptTsEngine   <-> TypeScript grammar/mutations
JavaScript/tsx  <-> JavaScriptTsxEngine  <-> TSX grammar/mutations
```

Engines must not perform dialect resolution from `Target.language`, path extension, CLI flags, or config. By the time `mutate()` is called, dialect selection is already complete.

## Current relationships in the codebase

### Current file extension relationships

A file extension generally maps to one language family:

| Extension | Family | Dialect selected by extension? |
|---|---|---|
| `.rs` | Rust | N/A |
| `.go` | Go | N/A |
| `.sol` | Solidity | N/A |
| `.cpp` / related C++ extensions | C++ | N/A |
| `.js` | JavaScript | `JavaScript/js` |
| `.jsx` | JavaScript | `JavaScript/jsx` |
| `.ts` | JavaScript | `JavaScript/ts` |
| `.tsx` | JavaScript | `JavaScript/tsx` |
| `.move` | Move | No. Ambiguous between Sui/IOTA/Aptos. |

So:

- `extension -> family` is usually one-to-one.
- `family -> extension` is often one-to-many.
- JavaScript extensions also identify dialect.
- Move's `.move` extension identifies only the family, not the dialect.

### Current engine relationships

Currently, engines are mostly one per language family:

| Family | Current engine |
|---|---|
| JavaScript | `JavaScriptLanguageEngine` |
| Move | `MoveLanguageEngine` |
| Rust | Rust engine |
| Go | Go engine |
| Solidity | Solidity engine |
| C++ | C++ engine |

For dialect-aware families, one engine handles many dialects:

```text
JavaScriptLanguageEngine
  -> JavaScript/js
  -> JavaScript/jsx
  -> JavaScript/ts
  -> JavaScript/tsx

MoveLanguageEngine
  -> Move/sui
  -> Move/iota
  -> Move/aptos
```

The concrete dialect is stored in `Target.language` and then reinterpreted inside the engine. This is the main behavior to eliminate. A concrete engine should already know its dialect; `Target.language` should be a storage/debug assertion, not an input to dialect selection.

### Current mutation relationships

`LanguageEngine::get_mutations()` currently returns one mutation catalog per engine.

JavaScript:

- `JavaScriptLanguageEngine::new()` adds `COMMON_MUTATIONS` and `JAVASCRIPT_MUTATIONS`.
- `src/languages/javascript/mutations.rs` currently has `JAVASCRIPT_MUTATIONS: &[Mutation] = &[]`.
- Therefore all JS dialects currently expose the same list: `COMMON_MUTATIONS`.
- JavaScript does not currently filter mutations by dialect.

Move:

- `MoveLanguageEngine::new()` adds `COMMON_MUTATIONS` and `MOVE_MUTATIONS`.
- During mutation, Move filters by dialect:

```rust
if !dialect_config.supports_mutation_slug(m.slug) {
    continue;
}
```

So the current true relationship is:

```text
engine -> one advertised mutation catalog
engine + dialect -> effective mutation set
```

This is awkward. It causes `print mutations` to special-case Move so it does not overreport unsupported inherited mutations.

### Current resolver relationships

Currently, each resolver is one per language family and many per dialect where applicable:

```text
JavaScriptLanguageResolver -> js/jsx/ts/tsx
MoveLanguageResolver       -> sui/iota/aptos
RustLanguageResolver       -> Rust only
```

This part is conceptually good.

The problem is that the resolver currently returns a canonical label, and the registry then maps that label back to a family engine. That leaves dialect-specific behavior split across resolver and engine.

## Problems to fix

### 1. Move dialect types are in core config

`src/core/types/config.rs` currently owns Move-specific types:

- `MoveDialectSetting`
- `MoveDialect`
- `MoveDialectSource`
- `ResolvedMoveDialect`
- `MoveLanguageConfig`

This is the clearest layering violation.

Core should not know what `sui`, `iota`, or `aptos` mean.

### 2. Dialect resolution is split between resolver and engine

For Move today:

```text
Move resolver resolves `Move/iota`
registry returns `MoveLanguageEngine`
MoveLanguageEngine reads `Target.language`
MoveLanguageEngine picks IOTA parser/syntax/filtering
```

For JavaScript today:

```text
JS resolver resolves `JavaScript/tsx`
registry returns `JavaScriptLanguageEngine`
JavaScriptLanguageEngine reads `Target.language`
JavaScriptLanguageEngine picks TSX parser
```

This means resolution is not actually complete when the resolver finishes.

Desired replacement:

```text
Move resolver resolves `.move` + config/CLI/default -> &MoveIotaEngine
MoveIotaEngine already owns IOTA grammar/syntax/mutation catalog
MoveIotaEngine::mutate(...) never asks "which Move dialect is this target?"

JavaScript resolver resolves `.tsx` -> &JavaScriptTsxEngine
JavaScriptTsxEngine already owns TSX grammar/mutation catalog
JavaScriptTsxEngine::mutate(...) never asks "which JS dialect is this target?"
```

Dialect resolution belongs only in the language family resolver and language-owned dialect helper modules. It should not appear in engine mutation loops.

### 3. Mutation catalogs are not concrete enough

Today Move has one engine catalog and dialect-specific filtering. This creates special cases in print commands:

```rust
fn mutation_is_available_for_label(language_label: &str, slug: &str) -> bool {
    if is_language_name(language_label) {
        return config_for_target_language(language_label).supports_mutation_slug(slug);
    }

    true
}
```

This should not live in `print mutations`, and it should not be Move-specific.

### 4. JavaScript should not get project-wide dialect config just to prove genericity

JavaScript dialects are safely and conventionally represented by file extension:

- `.js`
- `.jsx`
- `.ts`
- `.tsx`

A project-wide JavaScript dialect would be a footgun in mixed repos. If a file is named `.js` but needs JSX parsing, the best default answer is: fix the extension.

### 5. The current resolver trait is too split up

Current trait methods include:

```rust
engine()
is_language_name(...)
supports_cli_dialect_flag(...)
resolve_for_explicit_language(...)
resolve_for_explicit_dialect(...)
resolve_for_extension(...)
canonicalize_label(...)
expand_filter_labels(...)
```

This forces core to sequence pieces of resolution. The language module should make the language-specific decision in one place.

## New design model

### Core idea

```text
Resolver = language-family router and policy owner
Engine   = concrete resolved language/dialect mutator with no dialect-resolution logic
```

Examples:

```text
MoveLanguageResolver
  -> MoveSuiEngine
  -> MoveIotaEngine
  -> MoveAptosEngine

JavaScriptLanguageResolver
  -> JavaScriptJsEngine
  -> JavaScriptJsxEngine
  -> JavaScriptTsEngine
  -> JavaScriptTsxEngine
```

The concrete engines can be thin wrappers around shared family implementation code. We do **not** want four duplicated JavaScript mutators or three duplicated Move mutators.

But sharing implementation must not reintroduce family-level dialect resolution inside the engine. Shared code should receive an already-selected dialect configuration from the concrete engine, not derive one from `Target.language` or file extension.

### Relationship target

| Concept | Desired relationship |
|---|---|
| Resolver | 1 per language family |
| Resolver to dialects | 1 to many for dialect-aware families |
| Resolver to engines | 1 to many for dialect-aware families |
| Engine | 1 per concrete canonical label and 1 per dialect |
| Engine to dialect config | 1 fixed dialect config chosen at construction |
| Engine to mutations | 1 concrete effective mutation catalog |
| Engine to parser/syntax | 1 concrete parser/syntax configuration |
| `Target.language` | equals selected engine's `canonical_name()` |

### Concrete examples

```text
foo.tsx
  -> JavaScriptLanguageResolver
  -> JavaScriptTsxEngine
  -> engine.canonical_name() = "JavaScript/tsx"
  -> Target.language = "JavaScript/tsx"
  -> engine parser = TSX grammar
  -> engine mutations = common + javascript + typescript + jsx + tsx tiers, minus disabled ones

foo.move + --dialect iota
  -> MoveLanguageResolver
  -> MoveIotaEngine
  -> engine.canonical_name() = "Move/iota"
  -> Target.language = "Move/iota"
  -> engine parser = IOTA Move grammar
  -> engine syntax = IOTA Move syntax mapping
  -> engine mutations = common + move + iota tiers, minus disabled ones
```

### Dialect-resolution boundary

Dialect resolution may live in:

- `src/languages/<family>/resolver.rs`
- `src/languages/<family>/dialect.rs` parsing/validation helpers used by the resolver
- concrete engine constructors that receive an already-selected dialect enum and build fixed config

Dialect resolution must not live in:

- `src/core/`
- `LanguageEngine::mutate()` implementations
- shared family mutation helper functions
- `print mutations`
- store filtering code

Concrete forbidden patterns inside engines:

```rust
// Move: forbidden in mutate()
let dialect_config = config_for_target_language(&target.language);
let syntax = syntax_for_dialect(dialect_config.dialect);

// JavaScript: forbidden in mutate()
let dialect_config = config_for_language_name(&target.language)
    .unwrap_or_else(|| config_for_target_path(&target.path));
```

Allowed patterns:

```rust
// Resolver chooses the engine.
let engine = self.engine_for_dialect(resolved_dialect);

// Engine constructor freezes dialect config.
let engine = MoveDialectEngine::new(MoveDialect::Iota);

// Engine mutate uses only self-owned fixed config.
mutate_move_with_config(target, &self.config, self.syntax, &self.mutations)
```

## Mutation catalog tiers

We want mutation catalogs to be built in tiers:

```text
common mutations
  + language-family mutations
  + dialect-specific mutations
  - disabled inherited mutations
```

Examples:

```text
Move/sui
  = COMMON_MUTATIONS
  + MOVE_MUTATIONS
  + SUI_MOVE_MUTATIONS
  - SUI_DISABLED_MUTATIONS

Move/iota
  = COMMON_MUTATIONS
  + MOVE_MUTATIONS
  + IOTA_MOVE_MUTATIONS
  - IOTA_DISABLED_MUTATIONS

JavaScript/tsx
  = COMMON_MUTATIONS
  + JAVASCRIPT_MUTATIONS
  + TYPESCRIPT_MUTATIONS
  + JSX_MUTATIONS
  + TSX_MUTATIONS
  - TSX_DISABLED_MUTATIONS
```

This is forward-looking. Today JavaScript has no dialect-specific mutations, but this model makes adding them straightforward.

### Important consequence

`LanguageEngine::get_mutations()` should return the **exact effective catalog** for that concrete engine.

That means:

- `MoveIotaEngine::get_mutations()` does not include mutations disabled for IOTA.
- `JavaScriptTsxEngine::get_mutations()` can include TSX-only mutations.
- `print mutations` does not need Move-specific filtering.

## Proposed `LanguageEngine` contract

The existing trait is close:

```rust
pub trait LanguageEngine: Send + Sync {
    fn name(&self) -> &'static str;

    fn canonical_name(&self) -> &'static str {
        self.name()
    }

    fn get_mutations(&self) -> &[Mutation];

    fn mutate(&self, target: &Target) -> Vec<Mutant>;
}
```

### New stricter contract

Keep the trait mostly as-is, but strengthen the meaning of `canonical_name()`:

> `canonical_name()` must be storage-stable and concrete. For dialect-aware languages, it must include the dialect.

Also strengthen the responsibility boundary:

> `LanguageEngine::mutate()` must not resolve dialects. It may validate that `target.language` matches `self.canonical_name()` if useful, but it must not choose parser, syntax, mutation availability, or dialect config by parsing `target.language` or the file extension.

The resolver chooses the engine. The engine only applies its already-selected configuration.

Examples:

```text
Rust
Go
Solidity
C++
JavaScript/js
JavaScript/jsx
JavaScript/ts
JavaScript/tsx
Move/sui
Move/iota
Move/aptos
```

`name()` can remain display-oriented, e.g.:

```text
JavaScript
JavaScript JSX
TypeScript
TypeScript JSX
Sui Move
IOTA Move
Aptos Move
```

or it can mirror the canonical name if we want minimal churn.

### Engine implementation pattern

Use thin concrete engines that share common implementation.

For Move:

```rust
struct MoveDialectEngine {
    dialect: MoveDialect,
    canonical_name: &'static str,
    display_name: &'static str,
    config: MoveDialectConfig,
    mutations: Vec<Mutation>,
}
```

For JavaScript:

```rust
struct JavaScriptDialectEngine {
    dialect: JavaScriptDialect,
    canonical_name: &'static str,
    display_name: &'static str,
    parser: &'static TsLanguage,
    mutations: Vec<Mutation>,
}
```

Each implements `LanguageEngine`, but delegates most `mutate` logic to shared family functions.

The shared function must take the concrete configuration as an argument:

```rust
fn mutate_move_with_config(
    target: &Target,
    config: &MoveDialectConfig,
    syntax: MoveSyntax,
    mutations: &[Mutation],
) -> Vec<Mutant>;

fn mutate_javascript_with_config(
    target: &Target,
    parser: &'static TsLanguage,
    mutations: &[Mutation],
) -> Vec<Mutant>;
```

The shared function must not do this:

```rust
let config = config_for_target_language(&target.language);
let config = config_for_target_path(&target.path);
```

Those calls are dialect resolution. They belong in the resolver or in engine construction, not in `mutate()`.

## Proposed simplest `LanguageResolver` trait

The resolver should return a concrete engine. The selected engine's `canonical_name()` is the canonical language label to store.

This is the only place where target-level dialect selection happens. The resolver may inspect:

- explicit language labels such as `move/iota` or `javascript/tsx`
- path extension such as `.move` or `.tsx`
- CLI dialect, only for families that accept it
- raw config dialect, only for families that accept it
- language-owned defaults

The engine should receive none of those inputs except through its fixed construction-time fields.

```rust
pub trait LanguageResolver: Send + Sync {
    fn family(&self) -> &'static str;

    fn accepts_cli_dialect(&self) -> bool {
        false
    }

    fn resolve<'a>(
        &'a self,
        path: &Path,
        explicit_language: Option<&str>,
        cli_dialect: Option<&str>,
        config_dialect: Option<&str>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>>;

    fn filter_labels(&self, query: &str) -> Option<Vec<String>>;
}
```

### Justification for every method/property

#### `family()`

Needed for generic config lookup and diagnostics.

Core can look up:

```toml
[languages.move]
dialect = "sui"
```

by asking each resolver for its family key. Core does not need a `MoveLanguageConfig` field.

Examples:

```rust
MoveLanguageResolver.family() == "move"
JavaScriptLanguageResolver.family() == "javascript"
RustLanguageResolver.family() == "rust"
```

#### `accepts_cli_dialect()`

This is a per-language-family policy and does not need an input.

Move:

```rust
fn accepts_cli_dialect(&self) -> bool { true }
```

JavaScript:

```rust
fn accepts_cli_dialect(&self) -> bool { false }
```

Rationale:

- Move needs CLI dialect because `.move` is overloaded.
- JavaScript should not accept CLI dialect for now because extensions are safer.
- Core can generically reject or ignore unsupported global `--dialect` without knowing language names.

#### `resolve(...)`

This is the central operation. It replaces:

- `engine()`
- `is_language_name(...)`
- `resolve_for_explicit_language(...)`
- `resolve_for_explicit_dialect(...)`
- `resolve_for_extension(...)`
- most target-resolution uses of `canonicalize_label(...)`

Return meaning:

```rust
None
```

means this resolver does not claim the request.

```rust
Some(Ok(engine))
```

means this resolver claims the request and resolved it to a concrete engine. At this point dialect resolution is complete.

```rust
Some(Err(message))
```

means this resolver claims the request, but the request is invalid.

The engine's `canonical_name()` becomes the canonical label.

No resolution response object is needed because callers do not need to hold source metadata after resolution. Warnings can be logged while resolving.

Because resolution is centralized here, engines should not contain fallback logic such as "if `target.language` is unrecognized, infer from path". Unknown or conflicting dialect inputs should fail during resolution, before mutation starts.

#### `filter_labels(...)`

Filtering is not target resolution.

Target resolution must choose one concrete engine:

```text
move + configured iota -> Move/iota
```

Filtering often expands a family query to many concrete labels:

```text
move -> [Move/sui, Move/iota, Move/aptos]
javascript -> [JavaScript/js, JavaScript/jsx, JavaScript/ts, JavaScript/tsx]
```

So `filter_labels` remains as a separate method.

It replaces the current pair:

- `canonicalize_label(...)`
- `expand_filter_labels(...)`

because filtering should both canonicalize exact labels and expand family labels.

## Methods to remove from the current resolver trait

### Remove `engine()`

Current problem:

- A family resolver has one engine, but dialect-aware resolution should select a concrete engine dynamically.

Replacement:

- `resolve(...)` returns the concrete engine.

### Remove `is_language_name(...)`

Current problem:

- Core asks whether a resolver owns a label, then calls another method.
- This makes alias policy leak into core sequencing.

Replacement:

- `resolve(...)` either claims the request or returns `None`.
- `filter_labels(...)` either claims a filter query or returns `None`.

### Remove `resolve_for_explicit_language(...)`

Replacement:

- `resolve(...)` handles explicit language as one of its inputs.

### Remove `resolve_for_explicit_dialect(...)`

Replacement:

- `resolve(...)` handles CLI dialect only in context.

Bare dialects are ambiguous without a language/path context. If the CLI supports bare `--dialect`, core should pass it to candidate resolvers selected by explicit language or extension. If no context exists, core can error unless exactly one registered resolver accepts CLI dialect and can resolve it.

### Remove `resolve_for_extension(...)`

Replacement:

- `resolve(...)` handles path/extension as one input.

### Remove `canonicalize_label(...)`

Replacement:

- Use `resolve(...)` for target-like resolution to one engine.
- Use `filter_labels(...)` for query/filter contexts.

## Core config redesign

### Current bad state

Core config currently has:

```rust
pub struct LanguagesConfig {
    #[serde(rename = "move")]
    pub move_language: Option<MoveLanguageConfig>,
}
```

and Move-specific dialect enums/settings in core.

### Desired state

Core config should have a generic per-language map or table.

Conceptually:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LanguageConfig {
    pub dialect: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    pub languages: Option<HashMap<String, LanguageConfig>>,
}
```

Then:

```toml
[languages.move]
dialect = "iota"
```

is parsed generically by core and validated by `MoveLanguageResolver`.

### Language-owned config policy

Move:

- accepts `[languages.move].dialect`
- valid values: `sui`, `iota`, `aptos`

JavaScript:

- does not accept `[languages.javascript].dialect` for now
- if present, should error clearly, e.g.:

```text
JavaScript dialect config is not supported; use .js/.jsx/.ts/.tsx extensions.
```

Simple languages:

- do not accept dialect config
- if present, should error clearly.

### Config dialect lookup flow

Core should not parse dialect values. Core should fetch the raw string and pass it to the resolver.

Pseudo-code:

```rust
let config_dialect = config
    .languages
    .as_ref()
    .and_then(|langs| langs.get(resolver.family()))
    .and_then(|lang_cfg| lang_cfg.dialect.as_deref());
```

Then pass `config_dialect` into `resolver.resolve(...)`.

## Dialect input policies

### Move policy

Move resolution order:

```text
1. explicit dialect label, e.g. --language move/iota
2. CLI --dialect
3. [languages.move].dialect
4. default to sui, with warning
```

Notes:

- `.move` selects the Move family but not the dialect.
- If defaulting to Sui, log a warning during resolution.
- The warning does not require retaining a resolution source object.

Conflict behavior:

- `--language move/iota --dialect sui` should error.
- Exact message should tell users to use either profiled language label or `--dialect`, not both.

### JavaScript policy

JavaScript resolution order:

```text
1. explicit concrete label, e.g. --language javascript/tsx
2. file extension, e.g. .tsx
3. default to js only for ambiguous explicit family or extensionless cases, with warning if appropriate
```

For now:

- Reject CLI `--dialect` for JavaScript.
- Reject `[languages.javascript].dialect`.
- Do not let project-wide config override `.js/.jsx/.ts/.tsx`.

Rationale:

- JavaScript projects commonly mix dialects.
- File extension is per-file and standard.
- A project-wide dialect override is a footgun.
- If a file extension is wrong, fixing the extension is the preferred solution.

### Simple language policy

For Rust/Go/Solidity/C++:

- no dialects
- extension or explicit language resolves to a single concrete engine
- CLI/config dialect should error if routed to the language

## Registry responsibilities after refactor

The registry should be a generic router only. It should coordinate resolvers, but it should not know dialect names or perform dialect-specific selection itself.

### Target resolution pseudo-code

```rust
pub fn resolve_engine(
    &self,
    path: &Path,
    explicit_language: Option<&str>,
    cli_dialect: Option<&str>,
    config: &Config,
) -> Result<&dyn LanguageEngine, String> {
    for resolver in &self.resolvers {
        let config_dialect = config.dialect_for_family(resolver.family());
        let cli_dialect_for_resolver = if resolver.accepts_cli_dialect() {
            cli_dialect
        } else {
            None
        };

        if let Some(result) = resolver.resolve(
            path,
            explicit_language,
            cli_dialect_for_resolver,
            config_dialect,
        ) {
            return result;
        }
    }

    Err("No language resolver found".to_string())
}
```

Open detail: if global CLI `--dialect` is provided and a resolver does not accept CLI dialect, core must ensure it is not silently ignored. There are two reasonable approaches:

1. Prevalidate based on explicit language/path-selected candidate resolver.
2. Track whether any resolved target consumed the CLI dialect and error/warn if none did.

Prefer explicit errors over silent ignore.

### Loading targets

After resolution:

```rust
let engine = registry.resolve_engine(...)?;
let language = engine.canonical_name().to_string();
```

Then create:

```rust
Target { language, ... }
```

The selected engine is the source of truth. The target stores the canonical label for persistence, filtering, and diagnostics. It is not supposed to drive dialect selection later.

### Engine lookup by stored label

The store loads historical targets by `Target.language`. We still need to get an engine for a stored canonical label.

With per-dialect engines, registry can maintain a map:

```text
canonical label -> engine
```

Examples:

```text
"Move/iota" -> MoveIotaEngine
"JavaScript/tsx" -> JavaScriptTsxEngine
"Rust" -> RustEngine
```

This can be built when resolvers are registered, or provided by resolver methods.

Potential trait addition if needed:

```rust
fn engines(&self) -> Vec<&dyn LanguageEngine>;
```

But avoid adding this unless implementation needs it. An alternative is to make `filter_labels` and stored-label lookup go through `resolve(...)` with `explicit_language = Some(label)`.

If repeated lookup performance matters, registry can cache after resolving.

## Do we need `engines()`?

The minimal trait proposed earlier does not include `engines()`. However, current code has registry methods that need to enumerate languages/engines:

- `all_languages()` for `print mutations` with no language filter
- `get_engine(language_name)` for stored labels and print paths
- `get_mutation(language_name, slug)`
- `get_severity(language_name, slug)`

With per-dialect engines, enumeration may become important.

There are two options.

### Option A: keep the trait minimal and resolve labels on demand

Use:

```rust
resolver.resolve(Path::new("__virtual__"), Some(label), None, None)
```

for lookup by label.

Pros:

- Small trait.

Cons:

- Awkward virtual path.
- Harder to implement `all_languages()` cleanly.
- Resolver must handle label-only resolution everywhere.

### Option B: add `engines()`

Trait becomes:

```rust
pub trait LanguageResolver: Send + Sync {
    fn family(&self) -> &'static str;
    fn engines(&self) -> Vec<&dyn LanguageEngine>;
    fn accepts_cli_dialect(&self) -> bool { false }
    fn resolve<'a>(...) -> Option<Result<&'a dyn LanguageEngine, String>>;
    fn filter_labels(&self, query: &str) -> Option<Vec<String>>;
}
```

Pros:

- Registry can build canonical label maps cleanly.
- `all_languages()` is straightforward.
- Print-all behavior is straightforward.
- No virtual path hack.

Cons:

- One more method.

Recommendation: **add `engines()` if implementation pressure appears immediately.** It is probably worth it because current registry already has engine enumeration responsibilities.

If we include it, every method is justified:

- `family()` for config lookup.
- `engines()` for canonical lookup/enumeration.
- `accepts_cli_dialect()` for global CLI validation.
- `resolve(...)` for selecting one engine.
- `filter_labels(...)` for query expansion/canonicalization.

## Suggested final resolver trait

Given current registry needs, this is likely the practical simplest trait:

```rust
pub trait LanguageResolver: Send + Sync {
    fn family(&self) -> &'static str;

    fn engines(&self) -> Vec<&dyn LanguageEngine>;

    fn accepts_cli_dialect(&self) -> bool {
        false
    }

    fn resolve<'a>(
        &'a self,
        path: &Path,
        explicit_language: Option<&str>,
        cli_dialect: Option<&str>,
        config_dialect: Option<&str>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>>;

    fn filter_labels(&self, query: &str) -> Option<Vec<String>>;
}
```

If `Vec<&dyn LanguageEngine>` allocation is undesirable, use an iterator-like callback or slice of boxed engines internally. Start simple unless profiling says otherwise.

## Implementation notes by module

### `src/core/engine/traits.rs`

- Keep `LanguageEngine` mostly as-is.
- Strengthen doc comment for `canonical_name()`:
  - stable storage key
  - concrete dialect label when dialect-aware
  - one-to-one with the concrete engine
- Add a doc comment to `mutate()` stating that dialect resolution must already be complete.
- `mutate()` may assert/log if `target.language != self.canonical_name()`, but it must not use that mismatch to select a different dialect.
- Consider requiring implementors to override `canonical_name()` instead of defaulting to `name()` for dialect engines.

### `src/core/resolver.rs`

- Replace current split trait with the new resolver trait.
- Document that `resolve(...)` is the only target-resolution step that may choose dialect.
- Remove `ResolutionDefaults` and `DialectDefault` or shrink them away as part of config redesign.
- If a temporary compatibility layer is needed, keep old trait methods only during migration.

### `src/core/registry.rs`

Refactor from:

```text
resolver returns canonical string -> registry finds family engine -> engine re-resolves dialect
```

to:

```text
resolver returns concrete engine -> registry uses engine.canonical_name() -> engine mutates with fixed config
```

Likely methods after refactor:

```rust
register_resolver(...)
resolve_engine(...)
get_engine(canonical_or_alias: &str) -> Option<&dyn LanguageEngine>
filter_labels(query: &str) -> Vec<String>
all_languages() -> Vec<&str>
get_mutation(...)
get_severity(...)
```

`get_engine` should prefer exact canonical labels from concrete engines.

### `src/core/types/config.rs`

- Remove Move-specific config enums and structs.
- Introduce generic language config map.
- Preserve user-facing TOML syntax.
- Remove `resolve_move_dialect` from core.
- Remove `resolve_language_defaults` or replace it with generic raw config access.

Possible generic shape:

```rust
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LanguageConfig {
    pub dialect: Option<String>,
}

pub type LanguagesConfig = HashMap<String, LanguageConfig>;
```

Need to verify serde supports the desired TOML shape:

```toml
[languages.move]
dialect = "sui"
```

### `src/core/types/target.rs`

Current load paths use `ResolutionDefaults` and canonical language strings.

After refactor:

- Pass config or raw config dialect access to registry resolution.
- Store `engine.canonical_name().to_string()` in `Target.language`.
- Avoid Move-specific imports such as `dialect::is_language_name` in core target loading.

### `src/core/cmds/run.rs` / `mutate.rs` / `main_shared.rs`

- Remove Move-specific dialect default setup in command plumbing.
- Pass global CLI dialect to registry generically.
- Ensure `--dialect` is not silently ignored.
- Let resolver log default warnings when it must default.

### `src/core/cmds/print/mutations.rs`

Goal: remove Move-specific filtering.

Current bad pattern:

```rust
use crate::languages::r#move::dialect::{config_for_target_language, is_language_name};
```

After per-dialect engines:

- Resolve requested language/dialect to a concrete engine.
- Print `engine.get_mutations()` directly.
- No `mutation_is_available_for_label` function.

For no-language print:

- Iterate all concrete engines.
- This likely requires `resolver.engines()` and registry `all_languages()` returning concrete canonical labels.

### `src/languages/move/dialect.rs`

Move here from core:

- `MoveDialect`
- config parsing/validation helpers
- dialect aliases
- default dialect
- grammar config
- syntax config linkage
- disabled mutation slugs, or a mutation catalog builder

Important fix:

- Stop treating bare `move` as an unconditional alias for `Move/sui` in all contexts.
- In target resolution, `move` is a family selector that should use CLI/config/default policy.
- In filter context, `move` expands to all Move dialect labels.

### `src/languages/move/engine.rs`

Refactor from one `MoveLanguageEngine` into concrete dialect engines backed by shared code.

Current dialect-resolution code to remove from `mutate()`:

```rust
let dialect_config = config_for_target_language(&target.language);
let syntax = syntax_for_dialect(dialect_config.dialect);

for m in &self.mutations {
    if !dialect_config.supports_mutation_slug(m.slug) {
        continue;
    }
    ...
}
```

The replacement is one concrete engine per Move dialect:

```rust
pub struct MoveDialectEngine {
    dialect: MoveDialect,
    canonical_name: &'static str,
    display_name: &'static str,
    config: MoveDialectConfig,
    syntax: MoveSyntax,
    mutations: Vec<Mutation>,
}

impl LanguageEngine for MoveDialectEngine {
    fn name(&self) -> &'static str { self.display_name }
    fn canonical_name(&self) -> &'static str { self.canonical_name }
    fn get_mutations(&self) -> &[Mutation] { &self.mutations }
    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        mutate_move_with_config(target, &self.config, self.syntax, &self.mutations)
    }
}
```

Shared function:

```rust
fn mutate_move_with_config(
    target: &Target,
    dialect_config: &MoveDialectConfig,
    syntax: MoveSyntax,
    mutations: &[Mutation],
) -> Vec<Mutant>
```

Rules:

- `MoveDialectEngine::new(MoveDialect::Iota)` may call `config_for_dialect(MoveDialect::Iota)`.
- `MoveLanguageResolver` may choose `&self.iota` from CLI/config/default/label/extension.
- `MoveDialectEngine::mutate()` must not call `config_for_target_language(&target.language)`.
- Mutation filtering by unsupported slug should happen while building `self.mutations`, not inside the mutation loop.
- `target.language` may be checked against `self.canonical_name()` for diagnostics, but it must not decide the dialect.

### `src/languages/move/resolver.rs`

Resolver owns concrete engines:

```rust
pub struct MoveLanguageResolver {
    sui: MoveDialectEngine,
    iota: MoveDialectEngine,
    aptos: MoveDialectEngine,
}
```

Resolution behavior:

- claim explicit labels `move`, `move/sui`, `move/iota`, `move/aptos`
- claim extension `.move`
- accept CLI dialect
- accept config dialect
- default to Sui with warning when needed
- return `&self.sui`, `&self.iota`, or `&self.aptos`

This resolver is the only Move component that combines the inputs:

```text
explicit label + CLI dialect + config dialect + extension + default
```

into a selected dialect engine. Once it returns an engine, no later code should re-open the dialect decision.

### `src/languages/javascript/dialect.rs`

Keep JS dialect enum here.

Current helpers are useful but should be adjusted:

- `dialect_from_language_name("javascript")` currently returns `JavaScriptDialect::JavaScript`.
- That conflates family selector with default dialect.
- Split family parsing from concrete dialect parsing.

Suggested helpers:

```rust
enum JavaScriptSelector {
    Family,
    Dialect(JavaScriptDialect),
}

fn parse_selector(raw: &str) -> Option<JavaScriptSelector>;
fn dialect_from_extension(ext: &str) -> Option<JavaScriptDialect>;
fn canonical_name(dialect: JavaScriptDialect) -> &'static str;
```

### `src/languages/javascript/engine.rs`

Refactor from one engine into concrete JS dialect engines backed by shared mutation code.

Current dialect-resolution code to remove from `mutate()`:

```rust
let dialect_config = config_for_language_name(&target.language)
    .unwrap_or_else(|| config_for_target_path(&target.path));
let tree = parse_source(source, dialect_config.parser_language())
```

The replacement is one concrete engine per JavaScript dialect:

```rust
pub struct JavaScriptDialectEngine {
    dialect: JavaScriptDialect,
    canonical_name: &'static str,
    display_name: &'static str,
    parser: &'static TsLanguage,
    mutations: Vec<Mutation>,
}

impl LanguageEngine for JavaScriptDialectEngine {
    fn name(&self) -> &'static str { self.display_name }
    fn canonical_name(&self) -> &'static str { self.canonical_name }
    fn get_mutations(&self) -> &[Mutation] { &self.mutations }
    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        mutate_javascript_with_config(target, self.parser, &self.mutations)
    }
}
```

Parser selection moves to engine construction:

```rust
match dialect {
    JavaScriptDialect::JavaScript | JavaScriptDialect::Jsx => tree_sitter_javascript,
    JavaScriptDialect::TypeScript => tree_sitter_typescript,
    JavaScriptDialect::Tsx => tree_sitter_tsx,
}
```

Rules:

- `JavaScriptLanguageResolver` chooses `&self.tsx` for `.tsx` or `javascript/tsx`.
- `JavaScriptDialectEngine::mutate()` must not call `config_for_language_name(&target.language)`.
- `JavaScriptDialectEngine::mutate()` must not call `config_for_target_path(&target.path)`.
- Extension fallback belongs in the resolver, not the engine.
- `target.language` may be checked against `self.canonical_name()` for diagnostics, but it must not decide the dialect.

Note: current code uses the JavaScript parser for both JS and JSX.

### `src/languages/javascript/resolver.rs`

Resolver owns concrete engines:

```rust
pub struct JavaScriptLanguageResolver {
    js: JavaScriptDialectEngine,
    jsx: JavaScriptDialectEngine,
    ts: JavaScriptDialectEngine,
    tsx: JavaScriptDialectEngine,
}
```

Resolution behavior:

- explicit concrete labels resolve directly:
  - `javascript/js`
  - `javascript/jsx`
  - `javascript/ts`
  - `javascript/tsx`
- extension resolves directly:
  - `.js` -> JS engine
  - `.jsx` -> JSX engine
  - `.ts` -> TS engine
  - `.tsx` -> TSX engine
- bare `javascript` or `js` explicit family can default to JS with warning if needed
- reject CLI dialect for now
- reject config dialect for now

This resolver is the only JavaScript component that maps labels/extensions to dialect engines. The engine must not contain path-extension fallback because that would duplicate resolver behavior and make explicit TSX/JSX selection harder to reason about.

Open alias question:

- `js` currently means JavaScript family/default JS dialect.
- In filter context, `js` currently expands to all JS dialects.
- Decide whether to preserve this. If yes, document it clearly.

## Parser grammar notes

Build script currently builds:

JavaScript family:

- `tree-sitter-javascript`
- `tree-sitter-typescript`
- `tree-sitter-tsx`

Move family:

- `tree-sitter-move-sui`
- `tree-sitter-move-iota`
- `tree-sitter-move-aptos`

Move Sui/IOTA grammars need symbol renaming in `build.rs` because they export the same base symbol.

This grammar asymmetry is external and should not drive core design.

## Store and filtering notes

The store no longer imports Move or JavaScript dialect helpers to normalize labels on read.

Current desired behavior:

- newly stored labels are concrete engine canonical names
- exact stored labels should already be normalized
- filtering uses registry/resolver expansion
- raw filter queries are included alongside expanded labels so legacy rows such as `move` remain filterable without core knowing Move semantics

Filtering should remain family-aware:

```text
move       -> Move/sui, Move/iota, Move/aptos
move/iota  -> Move/iota
javascript -> JavaScript/js, JavaScript/jsx, JavaScript/ts, JavaScript/tsx
javascript/tsx -> JavaScript/tsx
rust       -> Rust
```

This is why `filter_labels` remains separate from `resolve`.

## CLI UX notes

### `--dialect`

Move should support it:

```bash
mewt run path/to/pkg --dialect iota
mewt print mutations --language move --dialect aptos
```

JavaScript should not support it for now:

```bash
mewt run src --dialect tsx
```

should error or at least not silently reinterpret `.js` files as TSX.

Suggested error:

```text
--dialect is not supported for JavaScript; use .js/.jsx/.ts/.tsx extensions.
```

### Conflict examples

Error:

```bash
mewt print mutations --language move/iota --dialect sui
```

Suggested message:

```text
Use either --language move/<dialect> or --language move --dialect <dialect>, not both.
```

### Default warnings

Move default:

```text
Move dialect not explicitly set; defaulting to 'sui'. Use --dialect or [languages.move].dialect to select sui|iota|aptos explicitly.
```

JavaScript default for ambiguous explicit family/extensionless case, if supported:

```text
JavaScript dialect not explicit and no extension was available; defaulting to 'js'. Use a .js/.jsx/.ts/.tsx extension or an explicit language label.
```

Do not retain warning source metadata after resolution. Log during resolution.

## Test plan

### Resolver tests

Move:

- [x] `.move` + CLI `iota` -> `Move/iota`
- [x] `.move` + config `aptos` -> `Move/aptos`
- [ ] `.move` + no dialect -> `Move/sui` and warning path exercised
- [ ] `--language move/iota` -> IOTA engine
- [x] `--language move` + config `iota` -> IOTA engine
- [x] `--language move/iota --dialect sui` errors
- [x] invalid dialect errors list `sui`, `iota`, `aptos`

JavaScript:

- `.js` -> `JavaScript/js`
- `.jsx` -> `JavaScript/jsx`
- `.ts` -> `JavaScript/ts`
- `.tsx` -> `JavaScript/tsx`
- explicit `javascript/tsx` -> TSX engine
- CLI dialect rejected
- config dialect rejected
- invalid labels error clearly

Simple languages:

- extension resolves to concrete engine
- explicit language resolves to concrete engine
- dialect inputs rejected

### Engine tests

Move:

- each dialect engine reports concrete `canonical_name()`
- each dialect engine has a one-to-one dialect mapping
- each dialect engine uses expected parser grammar from construction-time config
- each dialect engine uses expected syntax mapping from construction-time config
- disabled inherited mutations do not appear in `get_mutations()`
- `mutate()` does not call target-language dialect resolution helpers
- changing `target.language` to a different Move dialect does not cause the engine to switch dialect; it either still uses its own config or fails/asserts consistently

JavaScript:

- each dialect engine reports concrete `canonical_name()`
- each dialect engine has a one-to-one dialect mapping
- each dialect engine uses expected parser grammar from construction-time config
- all current JS dialect engines expose the same common mutation catalog today
- add a test fixture proving a TSX-only mutation can be cataloged only in TSX when that exists
- `mutate()` does not fall back to path extension or `target.language` to choose parser
- changing `target.path` from `.tsx` to `.js` does not cause `JavaScriptTsxEngine` to switch parser; resolver selection is the only place extension matters

### Print mutation tests

- `print mutations --language move --dialect iota` prints IOTA effective catalog only
- `print mutations --language move/iota` prints same catalog
- no Move-specific filter function remains
- `print mutations --language javascript/tsx` prints TSX catalog
- print-all iterates concrete engines or groups intentionally by family if UX changes

### Store/filter tests

- stored target labels are concrete canonical labels
- filter `move` includes all Move dialects
- filter `move/iota` includes only IOTA
- filter `javascript` includes all JS dialects
- filter `javascript/tsx` includes only TSX
- legacy labels canonicalize if backward compatibility is required

## Migration strategy

### Phase 1: Generic config shape — done

- [x] Introduce generic `[languages.<family>].dialect` storage.
- [x] Move Move dialect enum/settings into `src/languages/move`.
- [x] Keep TOML compatibility for `[languages.move].dialect`.
- [x] Update user docs to describe generic family config and Move-owned dialect policy.

### Phase 2: Concrete dialect engines — done

- [x] Refactor Move into three concrete engines sharing implementation.
- [x] Refactor JavaScript into four concrete engines sharing implementation.
- [x] Give each concrete engine fixed construction-time dialect config, parser/syntax, canonical name, and effective mutation catalog.
- [x] Remove dialect lookup from `mutate()` implementations.
- [x] Strengthen `canonical_name()` usage and trait docs.

### Phase 3: Resolver trait refactor — done

- [x] Introduce new trait shape with `family()`, `engines()`, `accepts_cli_dialect()`, `resolve(...)`, and `filter_labels(...)`.
- [x] Update family resolvers so they select concrete engines directly.
- [x] Update registry routing.
- [x] Update target loading to store `engine.canonical_name()`.
- [x] Remove old split resolver methods.
- [x] Remove resolver-to-label-to-family-engine routing.

### Phase 4: Mutation catalog tiers — mostly done

- [x] Build effective per-dialect Move catalogs at engine construction time.
- [x] Remove dialect filtering from Move `mutate()`.
- [x] Make `print mutations` rely on `engine.get_mutations()`.
- [ ] Add shared catalog-builder helpers if more dialect-specific mutation tiers appear.
- [ ] Add future TSX-/JSX-specific catalog tests when JavaScript gains dialect-specific mutations.

### Phase 5: Cleanup, docs, and tests — current priority

- [x] Remove Move imports from core print/store/target/config implementation paths.
- [x] Genericize CLI/help/error text where possible.
- [x] Keep legacy stored labels filterable through generic registry expansion plus raw query matching.
- [x] Move remaining Move-specific config/default tests out of core where they are testing Move policy rather than generic config mechanics.
- [x] Update `docs/configuration.md` for generic language config and Move resolver-owned dialect validation.
- [x] Update `docs/language-resolution-contract.md` from the old Phase 1 normalized-selection model to the concrete-engine resolver model.
- [x] Update `docs/move-unification-baseline.md` to describe the current concrete Move engines.
- [x] Add focused resolver tests for Move, JavaScript, and simple languages.
- [x] Add CLI/registry error regression tests for unsupported/ambiguous `--dialect` use.
- [ ] Revisit whether `ResolutionDefaults` should be renamed or simplified after docs/tests settle.
- [ ] Run `just pre-commit` after each batch.

## Open design questions

### 1. Should `LanguageResolver` include `engines()`?

Likely yes, because current registry needs all engines for print-all and lookup by stored canonical label.

Minimal without it is possible but awkward.

Recommended practical trait:

```rust
pub trait LanguageResolver: Send + Sync {
    fn family(&self) -> &'static str;
    fn engines(&self) -> Vec<&dyn LanguageEngine>;
    fn accepts_cli_dialect(&self) -> bool { false }
    fn resolve<'a>(...) -> Option<Result<&'a dyn LanguageEngine, String>>;
    fn filter_labels(&self, query: &str) -> Option<Vec<String>>;
}
```

### 2. What should `print mutations` with no language print?

Options:

1. Print every concrete engine/dialect separately.
2. Group by family when catalogs are identical.
3. Keep current one-per-family output for simple UX.

Per-dialect engines make option 1 easiest and most precise.

### 3. Should JavaScript support explicit concrete language labels?

Probably yes:

```bash
mewt print mutations --language javascript/tsx
```

This is less footgunny than project-wide config because it is explicit and often used for introspection, not bulk target reinterpretation.

But using explicit `--language javascript/tsx` on a target path could still override extension if current CLI supports explicit language for target loading. Decide whether explicit concrete labels may override extension for JS. If allowed, document that it is an expert override. If not allowed, resolver should reject extension conflicts.

### 4. Should JavaScript `js` mean family or dialect?

Currently `js` behaves as JavaScript family/default JS dialect depending on context.

Potential policy:

- In target resolution, `js` means concrete `JavaScript/js`.
- In filter context, `javascript` means all JS dialects, but `js` means only `JavaScript/js`.

This would reduce ambiguity but may be a behavior change.

## Final desired mental model

Remember this:

```text
Core does not know dialect species.
Language family resolvers perform all dialect resolution.
Resolvers choose concrete engines.
Every concrete engine maps one-to-one to one canonical dialect label.
Engines never infer dialect from Target.language, path, CLI, or config.
Mutation catalogs are effective per engine.
Target.language is engine.canonical_name().
Filtering is separate from resolving.
Move config lives in Move.
JS remains extension-driven.
```

That model removes Move-specific core code, avoids JavaScript config footguns, and gives us a clean path to dialect-specific mutations such as Sui-only or TSX-only operators.
