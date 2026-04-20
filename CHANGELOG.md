# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 3.1.0 - 2026-04-20

### Added
- C++ language support (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx`)
- Sui Move language support (`.move`)
- New mutation operators:
  - `NR` (Negation Removal) across languages
  - `RDV` (Return Default Value) for Solidity and C++
  - `RCI` (Require Condition Inversion) for Solidity
  - `DAS`, `MR`, and `VR` for C++

### Changed
- Reworked mutation conformance tests into a shared, cross-language test harness
- Build and CI hardening updates (including pinned actions and improved release/test workflows)

### Fixed
- Improved child-process termination handling in the test runner
- Fixed skipped arithmetic-operator-shuffle (`AOS`) mutations
- Expanded and corrected Go `ER`/`CR` mutation coverage

## 3.0.1 - 2026-03-24

### Fixed
- Grammar cache directory now rooted in `OUT_DIR` instead of a relative `target/` path, fixing panics during Nix sandbox builds

## 3.0.0 - 2026-03-24

### Added
- `--severity` filter for `print mutants` and `results` commands (comma-separated: `high`, `medium`, `low`)
- `--mutation_type` now accepts comma-separated values for multi-type filtering
- Glob pattern support for all target path arguments (e.g., `**/*.rs`, `src/**/*.go`)
- `mewt purge --all` flag to purge every target in the database regardless of config rules
- `mewt mutate --verbose` flag to restore detailed per-mutant output during mutation generation
- `mewt print targets` now shows a rich table with columns: in_db, on_disk, included, hash, path, mutants

### Changed
- **BREAKING**: Global `--cwd` flag replaced with `--config`
  - Pass a path to the config file; the directory containing it becomes the working directory
  - Relative paths in the config are resolved from the config file's location
- **BREAKING**: `mewt purge` default behavior changed
  - Without `--target`, now purges targets absent from `[targets].include` or present in `[targets].ignore` (previously purged all targets)
  - Use `mewt purge --all` to purge every target unconditionally
- **BREAKING**: `mewt mutate` is quiet by default, showing per-target summaries instead of per-mutant lines
  - Use `--verbose` to restore the previous detailed output
- **BREAKING**: `LanguageEngine` trait simplified
  - `tree_sitter_language() -> TsLanguage` removed
  - `apply_all_mutations(&self, target: &Target) -> Vec<Mutant>` renamed to `mutate`
  - `get_all_slugs()` and `get_severity_by_slug()` default methods removed (derive from `get_mutations()` directly)
- Target CLI argument for `run` and `mutate` is now optional; falls back to `[targets].include` from config
- Targets are sorted consistently across all commands (`run`, `mutate`, `print`, `results`, `status`)

### Fixed
- Improved grammar caches for faster repeated runs
- Improved monorepo support for vendored tree-sitter grammars
- Security dependency updates: `rsa` (0.9.8 → 0.9.10), `bytes` (1.10.1 → 1.11.1)
- Dependency updates: `tree-sitter` (0.25.8 → 0.25.10), `toml` (0.8.23 → 0.9.6)

## 2.0.1 - 2026-02-05

### Changed
- Added support for Rust edition 2021 by replacing let-chains with nested if statements
- Configured clippy to allow `collapsible_if` for broader edition compatibility

## 2.0.0 - 2026-02-05

### Added
- `mewt print config` command to display the effective configuration
- Dedicated TypeScript and TSX grammars for improved parsing accuracy

### Changed
- **BREAKING**: Configuration system overhauled with unified CLI/file symmetry
  - Configuration now uses dotted notation for CLI flags (e.g., `--log.level`, `--test.cmd`, `--test.timeout`)
  - Config file structure reorganized with nested sections (`[log]`, `[targets]`, `[run]`, `[test]`)
  - Added support for per-target test rules via `[[test.per_target]]` array in config file
  - CLI overrides now replace (not merge) config file values
- **BREAKING**: Removed environment variable configuration support
  - Previously supported variables (`MEWT_LOG_LEVEL`, `MEWT_DB`, `MEWT_TEST_CMD`, etc.) are no longer recognized
- **BREAKING**: Removed `mewt print results` command
  - Use `mewt results` instead (promoted in v1.1.0)
- Status filtering is now case-insensitive for `--status` flag
- Improved filter implementation consistency across `print mutants` and `results` commands

### Fixed
- Percentage complete display in `status` command campaign summary
- Internal namespacing improvements in core module

## 1.1.0 - 2026-01-28

### Added
- Go language support for mutation testing
- JavaScript and TypeScript language support (including JSX files)
- `mewt status` command for campaign overview with per-file breakdown and aggregates
  - `--format` option: "table" (default) or "json"
- `mewt results` command (promoted from `print results` subcommand)
  - Enhanced filtering with `--status`, `--language`, `--mutation_type`, `--line`, `--file` options
  - SARIF output format support (`--format sarif`)
  - JSON and "ids" output formats
- `mewt test --ids-file` option to read mutant IDs from file or stdin (use `-` for stdin)
- JSON output format support for multiple commands:
  - `mewt print mutations --format json`
  - `mewt print targets --format json`
  - `mewt print mutants --format json`
- Enhanced filtering for `print results`, `print mutants`, and `results` commands:
  - `--status`: Filter by outcome status (Uncaught, TestFail, Skipped, Timeout)
  - `--language`: Filter by programming language
  - `--mutation_type`: Filter by mutation slug (e.g., ER, CR, BR)
  - `--line`: Filter by line number
  - `--file`: Filter by file path (substring match)
- `mewt print mutants` filtering options:
  - `--tested`: Show only mutants with test outcomes
  - `--untested`: Show only mutants without test outcomes
  - `--format ids`: Output just mutant IDs, one per line

### Changed
- `mewt test --ids` is now optional when using `--ids-file`
- Cleaner log output (removed info level prefix)

### Removed
- `BuildFail` outcome status (simplified outcome types)

## 1.0.0 - 2024-12-20

Initial release.
