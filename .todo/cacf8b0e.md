---
id: "cacf8b0e"
title: "Add glob pattern support to purge --target flag"
tags: ["enhancement", "ux", "purge"]
---

## Problem

The `mewt purge --target` command currently only supports exact path matching. If you accidentally mutate unwanted directories like `node_modules`, `vendor`, `target`, or `.git`, you'd need to purge each file individually, which is impractical when there could be hundreds or thousands of erroneously generated mutants.

## Solution Implemented

### 1. Glob Pattern Support

All target CLI parameters now support glob patterns:
- `mewt purge --target "node_modules/**"` - purge all targets in node_modules
- `mewt purge --target "**/test_*.rs"` - purge all test files matching pattern
- `mewt results --target "src/**/*.rs"` - show results for Rust files in src
- `mewt print mutants --target "**/*.go"` - show mutants for all Go files

The implementation automatically detects glob characters (`*`, `?`, `[`) and uses `globset` for pattern matching. If no glob characters are found, it falls back to exact path matching.

### 2. Smart Default Purge Behavior

Changed purge behavior to be more intelligent:

**Default (no flags)**: Purge targets that are:
- NOT in config `[targets].include` patterns, OR
- ARE in config `[targets].ignore` patterns

This makes purge operate on the "inverse" of what other commands operate on, cleaning up accidental mutations outside your intended scope.

**`--all` flag**: Purge ALL targets in the database regardless of config

**`--target <pattern>` flag**: Purge specific targets matching the path/glob pattern (can override config and purge included targets)

### 3. Config-Aware Target Filtering

Added helper functions to use config targets when no explicit CLI target is provided:
- `Target::filter_existing_by_patterns()` - Filter database targets by ResolvedTargets patterns
- `Target::filter_by_path_or_config()` - Use CLI target if provided, otherwise fall back to config

Commands that now use config targets by default:
- `mewt results` (if no --target)
- `mewt print mutants` (if no --target)

### Files Changed

- `src/core/cli.rs` - Updated help text and added `--all` flag to PurgeArgs
- `src/core/cmds/purge.rs` - Implemented glob matching and smart default behavior
- `src/core/types/target.rs` - Added glob support to `filter_by_path()` and new helper functions
- `src/core/cmds/results.rs` - Use config targets when no --target specified
- `src/core/cmds/print/mutants.rs` - Use config targets when no --target specified

### Example Usage

```bash
# Purge accidental mutations (not in config)
mewt purge

# Purge all targets matching a glob
mewt purge --target "node_modules/**"
mewt purge --target "**/vendor/*"

# Purge everything in database
mewt purge --all

# Show results for config targets
mewt results

# Show results for specific pattern
mewt results --target "src/**/*.rs"
```

### Testing

Manual testing confirmed:
- Glob patterns work correctly for purge, results, and print mutants
- Default purge behavior correctly identifies targets outside config scope
- `--all` flag purges all targets
- Config targets are used as fallback for results and print mutants
- All pre-commit checks pass
