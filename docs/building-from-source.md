# Building from source

[Back to README](../README.md)

## Via Nix

With Nix flakes enabled:

```bash
git clone https://github.com/trailofbits/mewt.git
cd mewt
nix develop --command bash -c 'just install-nix' # or 'direnv allow' then 'just build'
mewt --version
```

## Native toolchain

Requirements:
- Rust toolchain (via rustup)
- C toolchain (gcc/clang) and `make`
- `pkg-config`
- SQLite development headers (`libsqlite3-dev`/`sqlite`)

Install common prerequisites:

### macOS (Homebrew)

```bash
# Command Line Tools (if not already installed)
xcode-select --install || true

brew install rustup-init sqlite pkg-config
rustup-init -y
source "$HOME/.cargo/env"
```

### Ubuntu/Debian

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libsqlite3-dev curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

### Build and run

```bash
cargo build --release
./target/release/mewt --help
```

Optional (install into your cargo bin):

```bash
cargo install --path . --locked --force
mewt --version
```
