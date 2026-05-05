
# Mewt

Mewt is a mutation testing tool. Mutation testing works by making small
changes (mutations) to your source code — like replacing `+` with `-` or
swapping `true` for `false` — and then running your test suite against each
change. If your tests still pass after a mutation, that's a gap: the mutant
"survived," meaning your tests didn't catch the change.

This tells you something code coverage alone can't: not just whether your
tests *execute* a line, but whether they'd actually *fail* if that line were
wrong.

**Supported languages:**
- C++
- Go
- JavaScript/TypeScript
- Rust
- Solidity
- Move (dialects: `sui`, `iota`; use `move`, `move/sui`, or `move/iota`)

For details on how campaigns work under the hood, see
[How it works](docs/how-it-works.md). For the language/dialect resolver contract,
see [Language resolution contract](docs/language-resolution-contract.md).
To add support for a new language, see [Adding a language](docs/adding-a-language.md).
For mutation test suite structure and shared test helper conventions, see
[`tests/README.md`](tests/README.md).

## Installation

### Prebuilt binaries (recommended)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/trailofbits/mewt/releases/latest/download/mewt-installer.sh | sh
```

Prebuilt installers are currently provided for macOS (aarch64) and Linux (x86_64).
Windows is currently unsupported.

To build from source instead, see [Building from source](docs/building-from-source.md).

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

- List Move mutation slugs for a specific dialect:

```bash
mewt print mutations --language move --dialect iota
```

- Print all mutants for a target path:

```bash
mewt print mutants --target path/to/contract.rs
```

- Show mutation test results (optionally filtered by target):

```bash
mewt results --target path/to/contract.rs
```

- Test all mutants even if more severe ones were uncaught (disable skip optimization):

```bash
mewt run path/to/contract.rs --comprehensive
```

## Configuration

Mewt reads configuration from the nearest `mewt.toml` found by walking up from the current working directory. CLI flags override config file values.

You can also point to an explicit config file with `--config path/to/mewt.toml`.

See [Configuration](docs/configuration.md) for the full reference and [`src/example.toml`](src/example.toml) for a commented example.

## Choosing a Move dialect

For `.move` files, mewt resolves dialect in this order:
1. `--dialect`
2. `[languages.move].dialect` in `mewt.toml`
3. default `sui`

Examples:

```bash
# Explicit dialect from CLI
mewt run path/to/package --dialect iota

# Print mutations for Move with explicit dialect
mewt print mutations --language move --dialect sui
```

Compatibility note:
- Use canonical Move language names: `move`, `move/sui`, and `move/iota`.
- Legacy names such as `SuiMove`, `sui_move`, and `suimove` are not supported.

## Examples

This repo includes example files you can try:

- C++: `tests/cpp/example.cpp`
- Go: `tests/go/example.go`
- JavaScript/TypeScript/JSX/TSX: `tests/javascript/example.js` (plus `example.ts`, `example.jsx`, `example.tsx`)
- Rust: `tests/rust/example.rs`
- Solidity: `tests/solidity/example.sol`
- Move: `tests/sui_move/example.move` (path retained for compatibility during migration)

## Notes

- Mixed-language projects are supported. When a directory is targeted, only files with supported extensions are considered.
