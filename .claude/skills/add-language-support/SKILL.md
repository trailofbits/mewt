---
name: add-language-support
description: Guides through implementing mutation testing support for new programming languages using tree-sitter grammars. Use when user asks to add support for a language such as Python, JavaScript, or any programming language, or mentions "add language support" or "new language".
---

# Adding a Language to Mewt

Guides you through adding a new programming language to mewt's mutation testing framework using tree-sitter grammars and the LanguageEngine trait.

## Architecture Overview

Each language implementation consists of:
- Tree-sitter grammar (C parser from grammar definitions)
- Language engine implementing LanguageEngine trait
- Syntax definitions mapping grammar node/field names
- Optional language-specific mutations
- Tests and examples

## 6-Phase Implementation

1. Grammar Acquisition
2. Build System Integration
3. Language Engine Implementation
4. Language Registration
5. Tests and Examples
6. Validation

---

## Phase 1: Grammar Acquisition

**CRITICAL**: This phase is the foundation. All subsequent work builds on having a valid tree-sitter grammar. Do not proceed without one.

### Step 0: Locate Tree-Sitter Grammar Repository

If the user has not provided a tree-sitter grammar repository URL, **STOP and ask**:

```
I need the tree-sitter grammar repository URL for <language>.

Do you know where it is? Common patterns:
- https://github.com/tree-sitter/tree-sitter-<language>
- https://github.com/<maintainer>/tree-sitter-<language>

If you don't know, I can search the web to find it. Would you like me to:
1. Search for the official tree-sitter grammar
2. Wait for you to provide the URL
```

If searching the web, look for:
- Official tree-sitter organization repos
- Well-maintained community grammars with recent activity
- Repos with generated `parser.c` files (check for `src/parser.c` in the repo)

**If no adequate grammar is found**: HALT and inform the user. Do NOT attempt to write a grammar from scratch. Tree-sitter grammars require specialized expertise.

### Step 1: Configure Grammar Source

Once you have the repository URL, edit `grammars/update.sh` and add to configuration arrays (lines 15-23):

```bash
declare -A REPO_URLS=(
  ["rust"]="https://github.com/tree-sitter/tree-sitter-rust"
  ["solidity"]="https://github.com/JoranHonig/tree-sitter-solidity"
  ["<language>"]="<tree-sitter-repo-url>"
)

declare -A GRAMMAR_PATHS=(
  ["rust"]="" # repo root
  ["solidity"]="" # repo root
  ["<language>"]="" # repo root or subdirectory if grammar is nested
)
```

### Step 2: Clone and Extract Grammar

Run the update script:

```bash
cd grammars
bash update.sh <language> true   # dry run to preview
bash update.sh <language> false  # actual update
```

The script clones the repository to a temp directory, extracts `parser.c`, `scanner.c` (if present), tree_sitter headers, and places them in `grammars/<language>/src/`.

**If the script fails**: Clone the repo manually to a temp directory, explore its structure to locate the parser files, then copy them into `grammars/<language>/src/`. The required files are:
- `src/parser.c` (mandatory)
- `src/scanner.c` (optional, language-dependent)
- `src/tree_sitter/` (header directory)
- `grammar.js` (optional but helpful)
- `src/node-types.json` (mandatory for syntax mappings)

### Validation

```bash
ls -la grammars/<language>/src/
# Must show: parser.c, tree_sitter/
# May show: scanner.c
```

---

## Phase 2: Build System Integration

Edit `build.rs` and add a build_grammar call for your language after line 57:

```rust
fn main() {
    // ... existing grammars ...

    // Build <Language> grammar
    let <language>_dir: PathBuf = ["grammars", "<language>", "src"].iter().collect();
    build_grammar(&<language>_dir, "tree-sitter-<language>");
}
```

The library name must follow `tree-sitter-<language>` convention and be unique.

### Validation

```bash
just check
# or
cargo check
```

You should see compilation of the new grammar library.

---

## Phase 3: Language Engine Implementation

Create directory: `mkdir -p src/languages/<language>`

**File: `src/languages/<language>/mod.rs`**
```rust
pub mod engine;
pub mod kinds;
pub mod syntax;
```

**File: `src/languages/<language>/syntax.rs`**

Map node/field names from `grammars/<language>/src/node-types.json`:

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

**File: `src/languages/<language>/kinds.rs`**
```rust
use crate::types::Mutation;
pub const <LANGUAGE>_MUTATIONS: &[Mutation] = &[];
```

**File: `src/languages/<language>/engine.rs`**

Use `src/languages/rust/engine.rs` as template. Key structure:

```rust
use std::sync::OnceLock;
use tree_sitter::Language as TsLanguage;
use crate::{LanguageEngine, mutations::COMMON_MUTATIONS, patterns, types::{Mutant, Mutation, Target}};
use super::{kinds::<LANGUAGE>_MUTATIONS, syntax::{fields, nodes}};

static <LANGUAGE>_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_<language>() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct <Language>LanguageEngine {
    mutations: Vec<Mutation>,
}

impl <Language>LanguageEngine {
    pub fn new() -> Self {
        let mut mutations = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(<LANGUAGE>_MUTATIONS);
        Self { mutations }
    }

    fn parse(&self, source: &str) -> Option<tree_sitter::Tree> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&self.tree_sitter_language()).ok()?;
        parser.parse(source, None)
    }
}

impl LanguageEngine for <Language>LanguageEngine {
    fn name(&self) -> &'static str { "<Language>" }
    fn extensions(&self) -> &[&'static str] { &["<ext>"] }
    fn tree_sitter_language(&self) -> TsLanguage {
        <LANGUAGE>_LANGUAGE
            .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_<language>()) })
            .clone()
    }
    fn get_mutations(&self) -> &[Mutation] { &self.mutations }

    fn apply_all_mutations(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;
        let tree = match self.parse(source) { Some(t) => t, None => return Vec::new() };
        let root = tree.root_node();
        let mut all_mutants = Vec::new();

        for m in &self.mutations {
            match m.slug {
                "ER" => all_mutants.extend(
                    patterns::replace(root, source,
                        &[nodes::EXPRESSION_STATEMENT, nodes::RETURN_STATEMENT],
                        "assert!(false);",  // Use language-appropriate error
                        &|node, src| !crate::utils::node_text(node, src).contains("assert!("))
                    .into_iter().map(|p| Mutant::from_partial(p, target, "ER"))
                ),
                "IF" => all_mutants.extend(
                    patterns::replace_condition(root, source,
                        nodes::IF_STATEMENT, fields::CONDITION, &["if"], "false")
                    .into_iter().map(|p| Mutant::from_partial(p, target, "IF"))
                ),
                // See src/languages/rust/engine.rs for complete examples of:
                // CR, IT, WF, AS, LC, BL, AOS, BOS, LOS, COS, SOS, AAOS, BAOS, SAOS
                _ => {}
            }
        }
        all_mutants
    }
}

impl Default for <Language>LanguageEngine {
    fn default() -> Self { Self::new() }
}
```

Reference `src/languages/rust/engine.rs` for complete mutation implementations.

---

## Phase 4: Language Registration

### Add Module to languages/mod.rs

Edit `src/languages/mod.rs`:

```rust
pub mod rust;
pub mod solidity;
pub mod <language>;  // Add this
```

### Register in main.rs

Edit `src/main.rs`, find the language registry section, and add:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = LanguageRegistry::new();
    registry.register(mewt::languages::rust::engine::RustLanguageEngine::new());
    registry.register(mewt::languages::solidity::engine::SolidityLanguageEngine::new());
    registry.register(mewt::languages::<language>::engine::<Language>LanguageEngine::new());

    run_main(Arc::new(registry)).await?;
    Ok(())
}
```

---

## Phase 5: Tests and Examples

Create `tests/<language>/examples/hello-world.<ext>` with diverse syntax: if/else, loops, function calls, variables, returns, booleans.

**File: `tests/<language>_tests.rs`**
```rust
mod <language> {
    mod integration_tests;
}
```

**File: `tests/<language>/integration_tests.rs`**

Use `tests/rust/integration_tests.rs` as template. Key pattern:

```rust
use mewt::{LanguageEngine, languages::<language>::engine::<Language>LanguageEngine, types::Target};
use tempfile::tempdir;

fn create_test_target(content: &str) -> (tempfile::TempDir, Target) {
    let temp_dir = tempdir().expect("Failed to create temp directory");
    let file_path = temp_dir.path().join("test.<ext>");
    std::fs::write(&file_path, content).expect("Failed to write test file");
    (temp_dir, Target {
        id: 1,
        path: file_path,
        file_hash: mewt::types::Hash::digest(content.to_string()),
        text: content.to_string(),
        language: "<Language>".to_string(),
    })
}

#[test]
fn test_basic_mutations() {
    let source = "if (true) { return 42; }";
    let (_temp_dir, target) = create_test_target(source);
    let engine = <Language>LanguageEngine::new();
    let mutants = engine.apply_all_mutations(&target);
    assert!(!mutants.is_empty(), "Should generate mutations");
}
```

---

## Phase 6: Validation

### Type Checking

```bash
just check
```

Ensure all code compiles without warnings.

### Build Mewt

```bash
cargo build --release
```

The language must be built into mewt before the `--language` flag will recognize it. You can run mewt directly without installing to PATH using `./target/release/mewt`.

### Verify Mutations

```bash
./target/release/mewt print mutations --language <language>
```

Should list all available mutations (common mutations from COMMON_MUTATIONS array).

### Generate Mutants

```bash
./target/release/mewt print mutants --target tests/<language>/examples/hello-world.<ext>
```

Verify:
- Reasonable number of mutants generated
- Mutants show clear variations (ER shows error replacement, etc.)
- Line numbers and positions are accurate

### Run Tests

```bash
just test
```

All tests should pass, including your new language tests.

---

## Common Pitfalls

### Node Type Mismatches

Always verify node names in `grammars/<language>/src/node-types.json`. Don't copy from documentation.

### FFI Function Naming

The tree-sitter binding must match the generated C function exactly: `tree_sitter_<language>`. Verify in generated parser header files.

### Missing scanner.c

Some grammars include external scanners (e.g., Rust). If build fails during C compilation, check if `grammars/<language>/src/scanner.c` exists. The build.rs `build_grammar()` function already handles this automatically.

### Incomplete Mutation Coverage

Start with basic patterns (ER, CR, IF/IT). Get common mutations working first, test thoroughly.

Most languages need zero language-specific mutations. Only add custom mutations for unique constructs (e.g., `ifnot`, specialized loop patterns) not covered by common mutations. Be conservative - new mutations that increase false positives harm the tool's signal-to-noise ratio.

### Parse Failures

If `mewt print mutants` generates no mutants:
- Verify grammar with `tree-sitter parse`
- Check test file syntax is correct
- Review parser output

---

## Success Checklist

- [ ] Grammar files in `grammars/<language>/src/`
- [ ] `build.rs` includes build_grammar call
- [ ] `src/languages/<language>/mod.rs` created
- [ ] `syntax.rs` has node/field mappings from node-types.json
- [ ] `kinds.rs` created
- [ ] `engine.rs` implements LanguageEngine trait
- [ ] `src/languages/mod.rs` includes module
- [ ] `src/main.rs` registers language
- [ ] Example files in `tests/<language>/examples/`
- [ ] Test modules created and pass `just test`
- [ ] `mewt print mutations --language <language>` works
- [ ] `mewt print mutants --target tests/<language>/...` generates mutants
- [ ] No build warnings with `just check`

---

## Quick Reference Example

Adding Go support (assuming tree-sitter-go URL is known):

```bash
# Phase 1: Grammar (after confirming URL: github.com/tree-sitter/tree-sitter-go)
# Edit grammars/update.sh, add go to REPO_URLS and GRAMMAR_PATHS
cd grammars
bash update.sh go false

# Phase 2: Build system
# Edit build.rs, add:
# let go_dir: PathBuf = ["grammars", "go", "src"].iter().collect();
# build_grammar(&go_dir, "tree-sitter-go");

# Phase 3: Engine
mkdir -p src/languages/go
# Create mod.rs, syntax.rs, kinds.rs, engine.rs (use rust as template)

# Phase 4: Registration
# Edit src/languages/mod.rs: pub mod go;
# Edit src/main.rs: registry.register(...GoLanguageEngine::new());

# Phase 5: Tests
mkdir -p tests/go/examples
# Create test file and integration_tests.rs

# Phase 6: Validate
just check
cargo build --release
./target/release/mewt print mutations --language go
./target/release/mewt print mutants --target tests/go/examples/hello-world.go
just test
```
