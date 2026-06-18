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
# dialect = "sui"             # Move resolver validates: sui, iota, aptos

[test]
# cmd = "cargo test"           # default test command
# timeout = 30                 # seconds; defaults to 2x baseline runtime

## Per-target overrides (first matching override for each section wins)
# [[per_target]]
# glob = "src/auth/login.rs"
# test.cmd = "cargo test auth_login"
# test.timeout = 60

# [[per_target]]
# glob = "sources/aptos/**/*.move"
# test.cmd = "aptos move test"
# languages.move.dialect = "aptos"
# run.mutations = ["ER", "CR"]
```

## CLI flags

CLI flags use dotted notation matching the config structure:
- `--db`
- `--log.level`, `--log.color`
- `--test.cmd`, `--test.timeout`

## Language dialect config

Language-specific settings live under `[languages.<family>]`. Core stores these settings generically; each language resolver decides whether a setting is valid and how to use it.

```toml
[languages.move]
dialect = "iota"
```

Dialect-aware languages accept configured dialects. Move uses this for `.move` files, which do not identify their dialect by extension. JavaScript usually selects dialects from `.js`, `.jsx`, `.ts`, and `.tsx` extensions, but config can override that when a project uses nonstandard extensions.

## Move dialect resolution order

For `.move` targets, the Move resolver resolves dialect in this order:
1. explicit language label, e.g. `move/iota`
2. top-level `[[per_target]]` `languages.move.dialect` for the target path
3. config `[languages.move].dialect`
4. default `sui`

Examples:

```bash
# Print mutation catalog for Move under sui dialect
mewt print mutations --language move/sui
```

Language selection:
- `--language move` is the Move family selector and uses config or the `sui` default.
- `--language move/sui`, `--language move/iota`, and `--language move/aptos` are canonical dialect selectors.
- Legacy aliases (`suimove`, `sui_move`, `SuiMove`) are not supported.

### Ignore flag

`--ignore` (CSV): comma-separated substrings; any target path containing any given value will be ignored.

Matching is substring-based, not glob-based. Example: `--ignore lib` excludes any path containing "lib". To be more specific, use `lib/`.
