---
id: "cacf8b0e"
title: "Add glob pattern support to purge --target flag"
tags: ["enhancement", "ux", "purge"]
---

## Problem

The `mewt purge --target` command currently only supports exact path matching. If you accidentally mutate unwanted directories like `node_modules`, `vendor`, `target`, or `.git`, you'd need to purge each file individually, which is impractical when there could be hundreds or thousands of erroneously generated mutants.

## Example Use Cases

```bash
# Purge all targets in node_modules
mewt purge --target "node_modules/**"

# Purge all targets matching a pattern
mewt purge --target "**/test_*.rs"

# Purge specific directory
mewt purge --target "vendor/*"
```

## Expected Behavior

1. If `--target` contains glob characters (`*`, `?`, `**`), treat it as a glob pattern
2. Match all targets whose paths match the glob pattern
3. Show all matching targets and ask for confirmation before purging
4. Maintain backward compatibility with exact path matching

## Implementation

1. Use the `glob` or `globset` crate for pattern matching
2. In `get_target_id_by_path`, detect if path contains glob characters
3. If glob detected, return multiple target IDs instead of Option<i64>
4. Refactor `execute_purge` to handle multiple target IDs
5. Update confirmation prompt to list all matching targets

## Files to Change

- `src/core/cmds/purge.rs` - Add glob matching logic
- `Cargo.toml` - Add glob dependency (likely already present)
- CLI help text for `--target` to document glob support

## Related

This would also be useful for other commands that accept target paths, like `results --target` and `print mutants --target`.
