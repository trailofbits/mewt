---
id: "ce822edb"
title: "Refactor all CSV-style arguments to use parse_csv utility"
tags: ["refactoring", "code-quality", "utilities"]
---

## Objective

Scan the codebase for all command-line arguments and config options that parse comma-separated values and refactor them to use the new `parse_csv<T>()` utility from `src/core/utils.rs`.

## ✅ COMPLETED

Successfully refactored all CSV parsing to use the `parse_csv` utility.

## Changes Made

### 1. `src/core/runner.rs` - Mutation slug filtering

**Before:**
```rust
let allowed_slugs: Option<Vec<String>> = filter_slugs.map(|s| {
    let slugs: Vec<String> = s.split(',').map(|s| s.trim().to_string()).collect();
    info!("Filtering mutations to test by slugs: {}", slugs.join(", "));
    slugs
});
```

**After:**
```rust
use crate::core::utils::parse_csv;

let allowed_slugs: Option<Vec<String>> = parse_csv::<String>(filter_slugs.as_deref())
    .inspect(|slugs| {
        info!("Filtering mutations to test by slugs: {}", slugs.join(", "));
    });
```

**Benefits:**
- 3 lines → 3 lines (but cleaner logic)
- Used `inspect()` instead of `map()` for side effects (clippy recommendation)
- Consistent error handling (filters invalid values)

### 2. `src/core/types/config.rs` - Target ignore patterns

**Before:**
```rust
let ignore = if let Some(cli_ign) = cli_ignore {
    cli_ign
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
} else {
    self.targets()
        .and_then(|t| t.ignore.clone())
        .unwrap_or_default()
};
```

**After:**
```rust
use crate::core::utils::parse_csv;

let ignore = if let Some(cli_ign) = cli_ignore {
    parse_csv::<String>(Some(cli_ign)).unwrap_or_default()
} else {
    self.targets()
        .and_then(|t| t.ignore.clone())
        .unwrap_or_default()
};
```

**Benefits:**
- 6 lines → 2 lines (-67% code)
- Handles empty values automatically
- Consistent with other CSV parsing

### 3. `src/core/types/config.rs` - Mutation resolution

**Before:**
```rust
pub fn resolve_mutations(&self, cli_mutations: Option<&str>) -> Option<Vec<String>> {
    cli_mutations
        .map(|s| {
            s.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .or_else(|| self.run().and_then(|r| r.mutations.clone()))
}
```

**After:**
```rust
pub fn resolve_mutations(&self, cli_mutations: Option<&str>) -> Option<Vec<String>> {
    parse_csv::<String>(cli_mutations)
        .or_else(|| self.run().and_then(|r| r.mutations.clone()))
}
```

**Benefits:**
- 6 lines → 2 lines (-67% code)
- More readable
- Consistent error handling

## Cases Where parse_csv Was NOT Used

### `src/core/cmds/test.rs` - Mutant ID parsing

**Reason:** Intentionally more flexible than CSV
- Accepts **whitespace** (spaces, tabs, newlines) OR commas
- Supports file input with newline-separated IDs
- Supports CLI input with comma-separated IDs
- Custom error handling (warns about invalid IDs)

**Code:**
```rust
for token in input.split(|c: char| c.is_whitespace() || c == ',') {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        continue;
    }
    match trimmed.parse::<i64>() {
        Ok(id) => ids.push(id),
        Err(_) => {
            warn!("Skipping invalid mutant ID: {}", trimmed);
        }
    }
}
```

This is the right choice - the flexibility requirement makes it unsuitable for simple CSV parsing.

## Summary

- ✅ **Refactored**: 3 CSV parsing locations
- ✅ **Removed**: ~20 lines of duplicate parsing code
- ✅ **Consistency**: All simple CSV parsing now uses the same utility
- ✅ **Tests**: All tests pass
- ✅ **Pre-commit**: All checks pass (fmt, check, lint, typos)
- ✅ **Documented**: One intentional exception (test.rs mutant IDs)

## Final Verification

```bash
# No more .split(',') outside of parse_csv itself
$ rg "\.split\(['\",']" --type rust -n | grep -v "src/core/utils.rs"
# (no output - all refactored!)

# All tests pass
$ cargo test
# test result: ok. 45 passed; 0 failed

# Pre-commit checks pass
$ just pre-commit
# ✅ All checks pass
```

## Impact

- **Code quality**: Centralized CSV parsing logic
- **Maintainability**: Single source of truth for CSV parsing
- **Testing**: CSV parsing is well-tested in utils.rs
- **Consistency**: All CSV parsing behaves identically
- **Error handling**: Invalid values filtered gracefully
- **Less duplication**: Removed ~20 lines of redundant code

The codebase now has consistent CSV parsing throughout, with the single intentional exception documented above.
