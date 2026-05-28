# Configuration

[Back to README](../README.md)

## Precedence

Configuration sources (highest to lowest priority):
1. CLI flags
2. Nearest `mewt.toml` found by walking up from the current working directory
3. Built-in defaults

Notes:
- CLI defaults are treated as built-in defaults (lowest); only flags explicitly provided override.
- Mutation slug whitelist overrides at the highest non-empty source; not merged.
- Ignore targets are merged additively across sources.

## Config file discovery

Starting from `cwd`, search for `mewt.toml` in that directory, then its parent, and so on, stopping at the first match.

## Example config

See [`src/example.toml`](../src/example.toml) for a fully commented example. The structure is:

```toml
## Database path (relative to this config file or absolute)
db = "{namespace}.sqlite"

[log]
level = "info"                # trace, debug, info, warn, error
# color = true                # omit for auto-detection

[targets]
# include = ["src/**/*.rs"]   # globs, files, and directories
# ignore = ["target", "node_modules", "vendor"]  # substring matching

[run]
# mutations = ["ER", "CR"]    # whitelist specific mutation slugs
# comprehensive = false        # test all mutants even if severe ones uncaught

[languages.move]
# dialect = "sui"             # one of: sui, iota, aptos

[test]
# cmd = "cargo test"           # default test command
# timeout = 30                 # seconds; defaults to 2x baseline runtime

## Per-target test overrides (first matching glob wins)
# [[test.per_target]]
# glob = "src/auth/*.rs"
# cmd = "cargo test --release -- --test-threads=1"
# timeout = 120
```

## CLI flags

CLI flags use dotted notation matching the config structure:
- `--db`
- `--log.level`, `--log.color`
- `--test.cmd`, `--test.timeout`
- `--dialect` (run, mutate, print mutations; Move only)

## Move dialect resolution order

For `.move` targets, mewt resolves dialect in this order:
1. CLI `--dialect`
2. Config `[languages.move].dialect`
3. Default `sui`

If no explicit dialect is selected, mewt defaults to `sui` and emits a warning when `.move` targets are processed.

Examples:

```bash
# Mutate using iota dialect for .move targets
mewt mutate src --dialect iota

# Print mutation catalog for Move under sui dialect
mewt print mutations --language move --dialect sui

# Equivalent canonical profiled selector
mewt print mutations --language move/sui
```

Language selection:
- `--language move` is the canonical Move family selector and uses `--dialect`, config, or the `sui` default.
- `--language move/sui`, `--language move/iota`, and `--language move/aptos` are canonical profiled selectors.
- For `print mutations`, use either `--language move/<dialect>` or `--language move --dialect <dialect>`, not both.
- Legacy aliases (`suimove`, `sui_move`, `SuiMove`) are not supported.

### Ignore flag

`--ignore` (CSV): comma-separated substrings; any target path containing any given value will be ignored.

Matching is substring-based, not glob-based. Example: `--ignore lib` excludes any path containing "lib". To be more specific, use `lib/`.
