---
id: "ce822edb"
title: "Refactor all CSV-style arguments to use parse_csv utility"
tags: ["refactoring", "code-quality", "utilities"]
---

## Objective

Scan the codebase for all command-line arguments and config options that parse comma-separated values and refactor them to use the new `parse_csv<T>()` utility from `src/core/utils.rs`.

## Background

We created a generic CSV parser utility for the `--severity` filter:
```rust
pub fn parse_csv<T>(csv_str: Option<&str>) -> Option<Vec<T>>
where
    T: FromStr,
{
    csv_str.map(|s| {
        s.split(',')
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    T::from_str(trimmed).ok()
                }
            })
            .collect()
    })
}
```

This should replace all hand-rolled CSV parsing throughout the codebase.

## Known CSV-Style Arguments

Search for these patterns to find candidates:

1. **CLI Arguments with "comma-separated" in help text:**
   ```bash
   rg "comma-separated" --type rust -B 2 -A 2
   ```

2. **Arguments that mention "CSV":**
   ```bash
   rg "CSV|csv" --type rust -B 2 -A 2
   ```

3. **Code that does `.split(',')`:**
   ```bash
   rg "\.split\(['\",']" --type rust -B 2 -A 2
   ```

## Example Refactoring Pattern

**Before:**
```rust
let mutations = cli_mutations
    .map(|s| {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
    .or_else(|| config.mutations.clone());
```

**After:**
```rust
use crate::core::utils::parse_csv;

let mutations = parse_csv::<String>(cli_mutations.as_deref())
    .or_else(|| config.mutations.clone());
```

## Likely Candidates

Based on the CLI structure, these are strong candidates:

1. **`--mutations` / `mutations` field** - Takes comma-separated mutation slugs
   - File: `src/core/cli.rs` - `RunArgs::mutations`, `MutateArgs::mutations`
   - Config: `src/core/types/config.rs` - `resolve_mutations()`
   
2. **`--ignore-targets` field** - Takes comma-separated ignore patterns
   - File: `src/core/cli.rs` - `RunArgs::ignore_targets`, `MutateArgs::ignore_targets`
   - Config: `src/core/types/config.rs` - `resolve_targets()`

3. **Config parsing** - Any config fields that parse comma-separated lists
   - Check `src/core/types/config.rs` for string splitting

4. **Test command parsing** - `--ids` parameter that takes comma-separated mutant IDs
   - File: `src/core/cli.rs` - `TestArgs::ids`
   - Implementation in test command

## Tasks

- [ ] Search for all `.split(',')` occurrences in the codebase
- [ ] Search for "comma-separated" in CLI help text
- [ ] For each occurrence:
  - [ ] Verify it's parsing CSV-style input
  - [ ] Check if the type implements `FromStr` (or can easily be made to)
  - [ ] Refactor to use `parse_csv<T>()`
  - [ ] Add/update tests if needed
- [ ] Look for any config TOML parsing that splits strings
- [ ] Document any cases where `parse_csv` can't be used and why

## Benefits

1. **Consistency** - All CSV parsing uses the same logic
2. **Error handling** - Invalid values are filtered out gracefully
3. **Testing** - CSV parsing logic is well-tested in one place
4. **Maintainability** - Future CSV parsing needs use the same utility
5. **Less code duplication** - Remove ~10-20 lines per occurrence

## Notes

- The utility filters out empty values automatically
- It requires types to implement `FromStr`
- For types that don't implement `FromStr`, may need to add a wrapper or keep custom parsing
