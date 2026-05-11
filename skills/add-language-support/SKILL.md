---
name: add-language-support
description: Implements mutation testing support for new programming languages using tree-sitter grammars and the LanguageEngine trait. Triggers on "add language support", "add [language] support", "implement [language]", "new language", or mentions of specific languages like Python, TypeScript, C++, Java.
license: Apache-2.0
metadata:
  author: trailofbits
  version: "1.1"
  mewt-version: ">=2.0.0"
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - Glob
  - AskUserQuestion
---

# Adding Language Support to Mewt

Implements mutation testing support for new programming languages using tree-sitter grammars and the LanguageEngine trait.

## When to Use

Use this skill when:
- Adding support for a new programming language (Python, JavaScript, Go, etc.)
- Implementing tree-sitter grammar integration
- User asks to "add language support" or "implement X language"
- Extending mewt's language capabilities

## When NOT to Use

**Don't use for using mewt:** If the user wants to USE mewt for mutation testing (not add language support), use the `mewt` skill instead.

**Don't use for grammar development:** This assumes a tree-sitter grammar already exists. If no grammar exists, that's a separate specialized task beyond this skill's scope.

## Essential Principles

**Grammar First**: Never attempt to write a tree-sitter grammar from scratch. Always find and verify an existing tree-sitter grammar repository before proceeding. Grammar development requires specialized expertise.

**API Simplicity**: The LanguageEngine trait has only 4 methods. Keep implementations minimal - no helper methods, just the trait interface and inline grammar loading.

**Resolver First (Dialect-Aware Families)**: If a language family has multiple dialects sharing an extension, resolution must be owned by the registry/resolver layer (precedence, canonical labeling, deterministic ambiguity handling). Do not add command-specific branching for final language+dialect selection.

**Canonical Labels Only**: For dialect-aware families, use canonical selectors/labels (e.g., `move`, `move/sui`, `move/iota`) and avoid introducing legacy alias compatibility unless explicitly requested.

**Common First**: Start with COMMON_MUTATIONS only. Add language-specific mutations only for unique constructs not covered by common patterns. Most languages need zero custom mutations.

**Per-Slug Mutation Tests**: Every mutation slug exposed by the engine must have a dedicated test module under `tests/<language>/mutations/<SLUG>.rs`. The guard test in `tests/languages.rs` enforces this convention. New languages must be wired into the guard before landing.

**Integration Conformance First**: Every language `tests/<language>/integration_tests.rs` must run the shared conformance harness from `tests/conformance.rs`, then add language-specific integration assertions as needed.

**Example Fixture Policy**: Keep canonical example fixtures at `tests/<language>/example.<ext>` (JavaScript may keep multiple canonical fixtures: `example.js`, `example.ts`, `example.jsx`, `example.tsx`). Integration tests should treat these as smoke checks (`!mutants.is_empty()`), while per-slug tests should use inline fixtures for precise assertions.

**Verification Required**: Each phase has explicit exit criteria. Do not proceed to the next phase until all validation passes.

## When to Use

- User explicitly requests adding support for a specific programming language
- User mentions "add language support" or "new language"
- User provides a tree-sitter grammar repository URL
- User asks how to extend mewt with new languages

## When NOT to Use

- Language already supported (check `src/languages/`) - use existing implementation
- User wants to modify existing language support - use general code editing
- User wants to add mutations to existing language - edit `src/languages/<lang>/mutations.rs`
- No tree-sitter grammar exists - inform user and halt (cannot create grammars)

## Linear Progression

### Phase 1: Grammar Acquisition

**Entry Criteria**: User has requested language support

**Actions**:

1. Check if tree-sitter grammar repository URL is provided
   - If not, ask user with common patterns:
     - `https://github.com/tree-sitter/tree-sitter-<language>`
     - `https://github.com/<maintainer>/tree-sitter-<language>`
   - Offer to search the web if needed

2. Once URL confirmed, use Edit to add to `grammars/update.sh` (lines 15-23):
   ```bash
   declare -A REPO_URLS=(
     ["<language>"]="<tree-sitter-repo-url>"
   )
   
   declare -A GRAMMAR_PATHS=(
     ["<language>"]=""  # or subdirectory if nested
   )
   ```

3. Run grammar extraction:
   ```bash
   cd grammars && bash update.sh <language> false
   ```

**Exit Criteria**:
- [ ] Grammar files exist in `grammars/<language>/src/`
- [ ] Must have: `parser.c`, `tree_sitter/` directory
- [ ] May have: `scanner.c`, `node-types.json`

### Phase 2: Build System Integration

**Entry Criteria**: Grammar files extracted successfully

**Actions**:

1. Use Edit to add build configuration to `build.rs` after line 57:
   ```rust
   // Build <Language> grammar
   let <language>_dir: PathBuf = ["grammars", "<language>", "src"].iter().collect();
   build_grammar(&<language>_dir, "tree-sitter-<language>");
   ```

2. Verify compilation:
   ```bash
   cargo check
   ```

**Exit Criteria**:
- [ ] `cargo check` completes without errors
- [ ] Grammar library compiles successfully

### Phase 3: Language Engine Implementation

**Entry Criteria**: Grammar builds successfully

**Actions**:

1. Create language module structure:
   ```bash
   mkdir -p src/languages/<language>
   ```

2. Write module declaration (`src/languages/<language>/mod.rs`):
   ```rust
   pub mod engine;
   pub mod mutations;
   pub mod syntax;
   ```

3. Create syntax mappings (`src/languages/<language>/syntax.rs`):
   - Use Read to examine `grammars/<language>/src/node-types.json`
   - Map node and field names from the grammar
   ```rust
   pub mod nodes {
       pub const IF_STATEMENT: &str = "if_statement";
       pub const RETURN_STATEMENT: &str = "return_statement";
       // Add more from node-types.json
   }
   
   pub mod fields {
       pub const CONDITION: &str = "condition";
       pub const ARGUMENTS: &str = "arguments";
   }
   ```
   - For every operator-focused slug (AOS, AAOS, BOS, BAOS, LOS, SAOS, COS), confirm which node kinds the grammar uses for those operators (e.g., `augmented_assignment_expression` versus `binary_expression`). Only plug a node kind into `patterns::shuffle_*` after verifying it in `node-types.json`, and include any language-specific operator tokens (such as Go's `&^=` or JavaScript's `**=`/`>>>=`).

4. Create mutations file (`src/languages/<language>/mutations.rs`):
   ```rust
   use crate::types::Mutation;
   pub const <LANGUAGE>_MUTATIONS: &[Mutation] = &[];
   ```

5. Implement engine (`src/languages/<language>/engine.rs`):
   - Use Read on `src/languages/rust/engine.rs` as reference
   - Follow the 4-method trait pattern:
   ```rust
   use std::sync::OnceLock;
   use tree_sitter::Language as TsLanguage;
   
   use crate::LanguageEngine;
   use crate::mutations::COMMON_MUTATIONS;
   use crate::patterns;
   use crate::types::{Mutant, Mutation, Target};
   use crate::utils::{node_text, parse_source};
   
   use super::mutations::<LANGUAGE>_MUTATIONS;
   use super::syntax::{fields, nodes};
   
   static <LANGUAGE>_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
   
   unsafe extern "C" {
       fn tree_sitter_<language>() -> *const tree_sitter::ffi::TSLanguage;
   }
   
   pub struct <Language>LanguageEngine {
       mutations: Vec<Mutation>,
   }
   
   impl <Language>LanguageEngine {
       pub fn new() -> Self {
           let mut mutations: Vec<Mutation> = Vec::new();
           mutations.extend_from_slice(COMMON_MUTATIONS);
           mutations.extend_from_slice(<LANGUAGE>_MUTATIONS);
           Self { mutations }
       }
   }
   
   impl LanguageEngine for <Language>LanguageEngine {
       fn name(&self) -> &'static str {
           "<Language>"
       }
   
       fn extensions(&self) -> &[&'static str] {
           &["<ext>"]
       }
   
       fn get_mutations(&self) -> &[Mutation] {
           &self.mutations
       }
   
       fn mutate(&self, target: &Target) -> Vec<Mutant> {
           let source = &target.text;
           
           // Load grammar once and cache it
           let language = <LANGUAGE>_LANGUAGE
               .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_<language>()) });
           
           let tree = match parse_source(source, language) {
               Some(t) => t,
               None => return Vec::new(),
           };
           let root = tree.root_node();
   
           let mut all_mutants = Vec::new();
           for m in &self.mutations {
               match m.slug {
                   "ER" => {
                       all_mutants.extend(
                           patterns::replace(
                               root,
                               source,
                               &[nodes::EXPRESSION_STATEMENT, nodes::RETURN_STATEMENT],
                               "panic!(\"mewt\")",  // Use language-appropriate error
                               &|node, src| !node_text(node, src).contains("panic!"),
                           )
                           .into_iter()
                           .map(|p| Mutant::from_partial(p, target, "ER")),
                       )
                   }
                   "IF" => {
                       all_mutants.extend(
                           patterns::replace_condition(
                               root,
                               source,
                               nodes::IF_STATEMENT,
                               fields::CONDITION,
                               &["if"],
                               "false",
                           )
                           .into_iter()
                           .map(|p| Mutant::from_partial(p, target, "IF")),
                       )
                   }
                   // Add more mutation patterns - see src/languages/rust/engine.rs
                   _ => {}
               }
           }
           all_mutants
       }
   }
   
   impl Default for <Language>LanguageEngine {
       fn default() -> Self {
           Self::new()
       }
   }
   ```

**Exit Criteria**:
- [ ] All 4 trait methods implemented
- [ ] Grammar loaded inline in `mutate()` (no helper methods)
- [ ] At least ER and IF mutations implemented
- [ ] Code compiles with `cargo check`

### Phase 4: Language Registration

**Dialect-aware note**: If the language can have multiple dialects under one extension, also wire/update registry resolver metadata and canonical labeling rules so command flows consume one resolved selection object.

**Entry Criteria**: Engine implementation compiles

**Actions**:

1. Use Edit to add module to `src/languages/mod.rs`:
   ```rust
   pub mod <language>;
   ```

2. Use Edit to register in `src/main.rs` (find the LanguageRegistry section):
   ```rust
   registry.register(mewt::languages::<language>::engine::<Language>LanguageEngine::new());
   ```

**Exit Criteria**:
- [ ] Module exported in `src/languages/mod.rs`
- [ ] Engine registered in `src/main.rs`
- [ ] `cargo build --release` succeeds

### Phase 5: Tests and Examples

For dialect-aware families that share an extension, add:
- Resolver integration tests proving deterministic selection for ambiguous extension paths.
- Dialect-divergence tests with at least one construct accepted by one dialect and rejected (or treated differently) by another.
- Filter/query tests using canonical labels/selectors (no implicit legacy alias guarantees).

**Entry Criteria**: Language builds and registers

**Actions**:

1. Scaffold the test directories:
   ```bash
   mkdir -p tests/<language>/mutations
   ```
   The mutation folder must contain one Rust module per slug (for example `tests/<language>/mutations/AOS.rs`).

2. Write canonical example fixture(s):
   - Most languages: `tests/<language>/example.<ext>`.
   - JavaScript-family languages: one canonical fixture per supported extension (`example.js`, `example.ts`, `example.jsx`, `example.tsx`) as needed.
   - Keep examples syntactically valid and representative, but small enough to stay smoke-test friendly.

3. Wire the language into the integration test entrypoint (`tests/languages.rs`):
   ```rust
   mod <language>;
   ```

4. Create `tests/<language>/mod.rs` to expose both integration and per-slug suites:
   ```rust
   mod integration_tests;
   mod mutations;
   ```

5. Author `tests/<language>/mutations/mod.rs` that re-exports every slug module:
   ```rust
   #![allow(non_snake_case)]

   #[path = "AAOS.rs"]
   mod aaos;
   #[path = "AOS.rs"]
   mod aos;
   // Repeat for every slug returned by engine.get_mutations()
   ```
   - Use uppercase filenames that match the slug (`<SLUG>.rs`) and map to lowercase module names via `#[path = ...]` where needed.
   - Keep manual slug wiring in `mutations/mod.rs` in sync with files on disk.

6. For each slug surfaced by `engine.get_mutations()`, create `tests/<language>/mutations/<SLUG>.rs`:
   - Prefer inline source fixtures for precise, slug-focused assertions.
   - Use integration helpers (`create_test_target`, shared slug assertion helpers) rather than duplicating setup code.
   - For operator families (AAOS/BAOS/SAOS/AOS/BOS/COS/LOS), cover all supported operators and include negative cases where useful.

7. Create `tests/<language>/integration_tests.rs` with this pattern:
   - Define a thin `create_test_target(...)` wrapper that delegates to `tests/utils.rs`.
   - Run `conformance::run_common_language_checks(...)` for baseline behavior.
   - Add canonical example-file smoke tests (`!mutants.is_empty()`).
   - Add only language-specific integration assertions beyond conformance (for example parser edge cases).

8. Update the guard in `tests/languages.rs` so the new language participates in per-slug coverage checks (import the engine and add a `check_language(...)` call).

**Exit Criteria**:
- [ ] Canonical example fixture(s) created at `tests/<language>/example.<ext>` (or JavaScript `example.*` set)
- [ ] Integration and per-slug test modules compiled into `tests/languages.rs`
- [ ] (Dialect-aware only) Resolver tests cover deterministic extension resolution and canonical dialect labeling
- [ ] (Dialect-aware only) Divergence tests assert meaningful behavior differences between dialects
- [ ] `tests/<language>/integration_tests.rs` runs `run_common_language_checks(...)`
- [ ] Every slug exposed by the engine has a dedicated test module under `tests/<language>/mutations`
- [ ] Guard test passes without missing or unexpected modules
- [ ] `cargo test` (or `just test`) passes the full suite

### Phase 6: Validation

**Entry Criteria**: All tests pass

**Actions**:

1. Build release binary:
   ```bash
   cargo build --release
   ```

2. Verify mutations are registered:
   ```bash
   ./target/release/mewt print mutations --language <language>
   ```
   - Should list COMMON_MUTATIONS (ER, CR, IF, etc.)

3. Generate and verify mutants:
   ```bash
   ./target/release/mewt print mutants --target tests/<language>/example.<ext>
   ```
   - Verify reasonable number of mutants
   - Check mutations are diverse (ER, IF, CR, etc.)
   - Verify line numbers are accurate

4. Run full test suite:
   ```bash
   just test
   ```

**Exit Criteria**:
- [ ] `cargo build --release` succeeds
- [ ] `mewt print mutations --language <language>` lists mutations
- [ ] `mewt print mutants` generates mutants from example file
- [ ] All tests pass with `just test`
- [ ] No warnings with `cargo check`

## API Quick Reference

### The 4-Method Trait

```rust
pub trait LanguageEngine: Send + Sync {
    fn name(&self) -> &'static str;              // Language name
    fn extensions(&self) -> &[&'static str];     // File extensions
    fn get_mutations(&self) -> &[Mutation];      // Available mutations
    fn mutate(&self, target: &Target) -> Vec<Mutant>;  // Generate mutants
}
```

**Key changes from older versions:**
- ✅ `mutate()` replaces `apply_all_mutations()`
- ✅ No `tree_sitter_language()` method - load grammar inline
- ✅ No helper methods - keep implementations minimal

### Common Mutation Patterns

Use Read on `src/languages/rust/engine.rs` for complete examples of:

| Slug | Pattern | Purpose |
|------|---------|---------|
| ER | `patterns::replace()` | Replace statements with errors |
| CR | `patterns::replace()` | Replace statements with comments |
| IF | `patterns::replace_condition()` | Replace if conditions with false |
| IT | `patterns::replace_condition()` | Replace if conditions with true |
| WF | `patterns::replace_condition()` | Replace while conditions with false |
| LC | `patterns::swap_branches()` | Swap true/false branches |
| BL | `patterns::replace()` | Replace boolean literals |
| AOS | `patterns::replace_operator()` | Replace arithmetic operators |
| BOS | `patterns::replace_operator()` | Replace bitwise operators |
| LOS | `patterns::replace_operator()` | Replace logical operators |
| COS | `patterns::replace_operator()` | Replace comparison operators |

## Common Pitfalls

### Node Type Mismatches

**Problem**: Using documentation names instead of actual grammar names  
**Solution**: Always verify in `grammars/<language>/src/node-types.json`

### FFI Function Naming

**Problem**: Incorrect external function name  
**Solution**: Must be exactly `tree_sitter_<language>` (all lowercase, underscores for hyphens)

### Missing Scanner

**Problem**: Build fails during C compilation  
**Solution**: Some grammars need `scanner.c` - the update script handles this automatically

### Over-Engineering

**Problem**: Adding helper methods, custom parsing logic  
**Solution**: Keep it minimal - just implement the 4 trait methods

### Too Many Custom Mutations

**Problem**: Adding language-specific mutations before verifying common ones work  
**Solution**: Start with COMMON_MUTATIONS only. Most languages need zero custom mutations.

### Compound Assignment Node Kinds

**Problem**: Assuming compound assignments share the `binary_expression` node kind  
**Solution**: Check `node-types.json` for the exact node kind (e.g., `augmented_assignment_expression`, `compound_assignment_expr`) and wire AAOS/BAOS/SAOS to that. Include all language-specific operator tokens when configuring `patterns::shuffle_operators`.

## Success Checklist

### Dialect-Aware Extension Checklist (apply when relevant)
- [ ] Resolver owns final language+dialect selection (no command-level branching)
- [ ] Canonical dialect labels/selectors are documented and used consistently
- [ ] Ambiguous extension handling is deterministic and test-covered
- [ ] Dialect-divergence behavior is explicit and test-enforced
- [ ] Store/filter behavior uses canonical labels without assuming legacy alias support

## Success Checklist

- [ ] Grammar files extracted to `grammars/<language>/src/`
- [ ] Build system updated in `build.rs`
- [ ] Module structure created (mod.rs, syntax.rs, mutations.rs, engine.rs)
- [ ] Syntax mappings verified from node-types.json
- [ ] Engine implements exactly 4 trait methods (no extras)
- [ ] Grammar loaded inline in `mutate()` using OnceLock
- [ ] Language registered in src/languages/mod.rs and src/main.rs
- [ ] Example files created with diverse syntax
- [ ] Integration tests written and passing
- [ ] `cargo build --release` succeeds without warnings
- [ ] `mewt print mutations --language <language>` works
- [ ] `mewt print mutants` generates mutants from examples
- [ ] All tests pass with `just test`

## Example: Adding Go Support

Condensed walkthrough assuming tree-sitter-go URL is known:

```bash
# Phase 1: Edit grammars/update.sh to add go configuration, then:
cd grammars && bash update.sh go false

# Phase 2: Edit build.rs to add build_grammar call for go

# Phase 3: Create language implementation
mkdir -p src/languages/go
# Create mod.rs, syntax.rs, mutations.rs, engine.rs

# Phase 4: Edit src/languages/mod.rs to export go module
# Edit src/main.rs to register GoLanguageEngine

# Phase 5: Create tests
mkdir -p tests/go/examples
# Create example.go and integration_tests.rs

# Phase 6: Validate
cargo build --release
./target/release/mewt print mutations --language go
./target/release/mewt print mutants --target tests/go/example.go
just test
```
