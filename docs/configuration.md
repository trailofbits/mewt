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

### Ignore flag

`--ignore` (CSV): comma-separated substrings; any target path containing any given value will be ignored.

Matching is substring-based, not glob-based. Example: `--ignore lib` excludes any path containing "lib". To be more specific, use `lib/`.
