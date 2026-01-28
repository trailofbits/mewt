# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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
