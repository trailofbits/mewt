
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
- Go
- JavaScript/TypeScript
- Rust
- Solidity

For details on how campaigns work under the hood, see
[How it works](docs/how-it-works.md). To add support for a new language, see
[Adding a language](docs/adding-a-language.md).

## Installation

### Prebuilt binaries (recommended)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/trailofbits/mewt/releases/latest/download/mewt-installer.sh | sh
```

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

See [Configuration](docs/configuration.md) for the full reference and [`src/example.toml`](src/example.toml) for a commented example.

## Examples

This repo includes example files you can try:

- Go: `tests/go/examples/hello-world.go`
- JavaScript/TypeScript: `tests/javascript/examples/simple.js`
- Rust: `tests/rust/examples/hello-world.rs`
- Solidity: `tests/solidity/examples/hello-world.sol`

## Notes

- Mixed-language projects are supported. When a directory is targeted, only files with supported extensions are considered.
