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

**Common First**: Start with COMMON_MUTATIONS only. Add language-specific mutations only for unique constructs not covered by common patterns. Most languages need zero custom mutations.

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

**Entry Criteria**: Language builds and registers

**Actions**:

1. Create test directory structure:
   ```bash
   mkdir -p tests/<language>/examples
   ```

2. Write example file (`tests/<language>/examples/hello-world.<ext>`):
   - Include diverse constructs: if/else, loops, functions, variables, returns
   - Keep it simple and valid syntax

3. Create test module file (`tests/<language>_tests.rs`):
   ```rust
   mod <language> {
       mod integration_tests;
   }
   ```

4. Create test file (`tests/<language>/integration_tests.rs`):
   ```rust
   use mewt::LanguageEngine;
   use mewt::languages::<language>::engine::<Language>LanguageEngine;
   use mewt::types::{Hash, Target};
   use std::fs;
   use tempfile::tempdir;
   
   fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
       let temp_dir = tempdir().expect("Failed to create temp directory");
       let file_path = temp_dir.path().join("test.<ext>");
       fs::write(&file_path, content).expect("Failed to write test file");
       
       (temp_dir, Target {
           id: 1,
           path: file_path.clone(),
           file_hash: Hash::digest(content.to_string()),
           text: content.to_string(),
           language: "<Language>".to_string(),
       })
   }
   
   #[test]
   fn test_basic_mutations() {
       let source = "if (true) { return 42; }";
       let (_temp_dir, target) = create_test_target(source);
       let engine = <Language>LanguageEngine::new();
       let mutants = engine.mutate(&target);
       assert!(!mutants.is_empty(), "Should generate mutations");
   }
   ```
   - Add a regression test that exercises every compound or augmented assignment form the language supports and assert that each related slug (AAOS, BAOS, SAOS) produces at least one mutant. Apply the same "every slug emits at least one mutant" check to any other mutation families that rely on non-trivial node kinds.

**Exit Criteria**:
- [ ] Example file created with diverse syntax
- [ ] Test module structure created
- [ ] At least one integration test written
- [ ] `cargo test` passes all tests

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
   ./target/release/mewt print mutants --target tests/<language>/examples/hello-world.<ext>
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
# Create hello-world.go and integration_tests.rs

# Phase 6: Validate
cargo build --release
./target/release/mewt print mutations --language go
./target/release/mewt print mutants --target tests/go/examples/hello-world.go
just test
```
