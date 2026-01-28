
# Mewt

`mewt` is a tool for running mutation testing campaigns against smart contracts and other code written in a variety of languages.

## Installation

### Prebuilt binaries (recommended)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/trailofbits/mewt/releases/latest/download/mewt-installer.sh | sh
```

### Build from source (via Nix)

With Nix flakes enabled:

```bash
git clone https://github.com/trailofbits/mewt.git
cd mewt
nix develop --command bash -c 'just install-nix' # or 'direnv allow' then 'just build'
mewt --version
```

### Build from source (native toolchain)

Requirements:
- Rust toolchain (via rustup)
- C toolchain (gcc/clang) and `make`
- `pkg-config`
- SQLite development headers (`libsqlite3-dev`/`sqlite`)

Install common prerequisites:

- macOS (Homebrew):

```bash
# Command Line Tools (if not already installed)
xcode-select --install || true

brew install rustup-init sqlite pkg-config
rustup-init -y
source "$HOME/.cargo/env"
```

- Ubuntu/Debian:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libsqlite3-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Build and run:

```bash
cargo build --release
./target/release/mewt --help
```

Optional (install into your cargo bin):

```bash
cargo install --path . --locked --force
mewt --version
```

## Quick start

- Mutate a single file (auto-detected language):

```bash
mewt run path/to/contract.rs
```

- Mutate all supported files in a directory (recursive):

```bash
mewt run path/to/project
```

- List available mutation slugs for a language:

```bash
mewt print mutations --language rust
```

- Print all mutants for a target path:

```bash
mewt print mutants --target path/to/contract.rs
```

- Show mutation test results (optionally filtered by target):

```bash
mewt print results --target path/to/contract.rs
```

- Test all mutants even if more severe ones were uncaught (disable skip optimization):

```bash
mewt run path/to/contract.rs --comprehensive
```

## Overview

This tool is designed to provide as pleasant a developer experience as possible while conducting mutation campaigns, which are notoriously messy and slow.

mewt operates on one single `mewt.sqlite` database, this stores the target files and mewt will reliably restore the original after a given mutation is tested, or after the campaign is interrupted with ctrl-c. However, this software is a work in progress so we strongly recommend running mutation campaigns against a clean git repo so that you can use `git reset --hard HEAD` to restore any mutations that escape the cleanup phase.

All target files are stored in the database and linked to a series of mutations. Each mutation is linked to one or zero outcomes. At the beginning of a mutation campaign, all targets are saved and all mutations are generated. This generally happens quickly, within a couple seconds.

Then, the real work begins: mewt will work through the list of target files, replacing it with a mutated version. For each mutated version, it will run the test command and save the outcome. If the mutation campaign is interrupted, it will pick up where it left off (unless the target file changed, in which case it will start over).

This may take a very long time. Assuming the tests take 1 minute to run, there are 10 files, and 100 mutants were generated for each, the runtime (*assuming zero mewt overhead*) will be 1 * 10 * 100 = 1000 minutes or 16 hours.

For this reason, making `mewt` run fast is not enough to conduct fast mutation campaigns. Instead, a few features make this process somewhat less painful:
- resume by default: if a campaign gets interrupted halfway through for whatever reason, we don't need to restart from the very beginning
- customizable targets: you can give mewt a directory as its `target` and it will mutate all supported files in this directory, which may take a long time. Or, you can give it one file and it will only mutate that file.
- skipping less severe mutants when more severe ones are uncaught: if replacing an expression with a `throw` statement is not caught by the test suite, this indicates the expression is never run by the test suite. Therefore, it's safe to assume that any other mutation to this line, will also not be caught by the test suite so subsequent mutations are skipped. This can drastically decrease the runtime against poorly tested code. However, this also means the runtime will increase after the test suite is improved and the mutation campaign starts testing parts of the code more deeply than it did before.

Tip: pass `--comprehensive` to `mewt run` to disable this optimization and test all mutants even when more severe ones on the same line are uncaught.

Despite these features, mutation campaigns are best conducted infrequently eg after an overhaul to the test suite rather than after adding each individual test. Therefore, mutation testing is not suitable for running in the CI after every push. You may want to run a campaign at the end of the day so that it can run overnight.

## Adding a language

The architecture is language-agnostic. To add a new language, follow these steps. Where possible, prefer using the grammar update script to automate vendor steps.

1) Vendor the grammar

- Add entries for your language to `mewt/grammars/update.sh` in both `REPO_URLS` and `GRAMMAR_PATHS`.
- Run the grammars update script

```bash
cd mewt/grammars
bash update.sh <language> true  # dry run
bash update.sh <language> false # actual update
```

You can also vendor manually by placing generated C sources under `mewt/grammars/<language>/src/` (must include `parser.c`) and `mewt/grammars/<language>/grammar.js`.

2) Build integration

- Extend `mewt/build.rs` to compile `mewt/grammars/<language>/src/parser.c` into a static library. Add a call to `build_grammar()` with your language directory and library name (see existing examples).

3) Language engine creation

- Create `mewt/src/languages/<language>/` directory with these files:
  - `mod.rs` - module declarations
  - `engine.rs` - implement `LanguageEngine` trait (copy and modify an existing engine)
  - `kinds.rs` - language-specific mutations (merged with common mutations)
  - `syntax.rs` - grammar node & field names from `grammars/<lang>/src/node-types.json`

4) Language registration

- Add your language module to `mewt/src/languages/mod.rs`
- Register the engine in `mewt/src/main.rs` by adding a `registry.register()` call

5) Tests and examples

- Add example files under `mewt/tests/<language>/examples/`
- Add tests under `mewt/tests/<language>/`

6) Validate

- `just check`
- `mewt print mutations --language <language>` shows your slugs
- `mewt print mutants --target mewt/tests/<language>/examples.` generates mutants for example files

## Configuration and precedence

Configuration sources (highest to lowest priority):
1. CLI flags
2. Environment variables
3. Nearest `mewt.toml` found by walking up from the current working directory
4. Built-in defaults

Notes:
- CLI defaults are treated as built-in defaults (lowest); only flags explicitly provided override.
- Mutation slug whitelist overrides at the highest non-empty source; not merged.
- Ignore targets are merged additively across sources.

Config file discovery: starting from `cwd`, search for `mewt.toml` in that directory, then its parent, and so on, stopping at the first match.

Example config:

```toml
[log]
level = "info"            # one of: trace, debug, info, warn, error
color = true               # optional boolean; omit for auto

[general]
db = "mewt.sqlite"
ignore_targets = ["build/", "node_modules/"]  # substring matches, not globs

[mutations]
slugs = ["ER", "CR"]      # global whitelist; overrides other sources if set/non-empty

[test]
cmd = "cargo test"
timeout = 120
```

Environment variables:
- `MEWT_LOG_LEVEL`: "debug", "info", "warn", etc
- `MEWT_LOG_COLOR`: "on" or "off" (omit for "auto")
- `MEWT_DB`: path to sqlite db file
- `MEWT_IGNORE_TARGETS`: CSV list of target substrings to ignore
- `MEWT_SLUGS`: CSV whitelist of slugs to mutate
- `MEWT_TEST_CMD`: command to run to assess mutants
- `MEWT_TEST_TIMEOUT`: timeout in seconds to use for tests

CLI:
- `--ignore` (CSV): comma-separated substrings; any target path containing any given value will be ignored.
  - Matching is substring-based, not glob-based. Example: `--ignore lib` excludes any path containing "lib". To be more specific, use `lib/`.

## Examples

This repo includes example contracts you can try:

- Go: `mewt/tests/go/examples/hello-world.go`
- Rust: `mewt/tests/rust/examples/hello-world.rs`
- Solidity: `mewt/tests/solidity/examples/hello-world.sol`

## Notes

- Mixed-language projects are supported. When a directory is targeted, only files with supported extensions are considered.
