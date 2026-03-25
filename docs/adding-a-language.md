# Adding a language

[Back to README](../README.md) | [Contributing guidelines](../CONTRIBUTING.md)

The architecture is language-agnostic. To add a new language, follow these steps. Where possible, prefer using the grammar update script to automate vendor steps.

## 1. Vendor the grammar

- Add entries for your language to `grammars/update.sh` in both `REPO_URLS` and `GRAMMAR_PATHS`.
- Run the grammars update script:

```bash
cd grammars
bash update.sh <language> true  # dry run
bash update.sh <language> false # actual update
```

You can also vendor manually by placing generated C sources under `grammars/<language>/src/` (must include `parser.c`) and `grammars/<language>/grammar.js`.

## 2. Build integration

- Extend `build.rs` to compile `grammars/<language>/src/parser.c` into a static library. Add a call to `build_grammar()` with your language directory and library name (see existing examples).

## 3. Language engine creation

- Create `src/languages/<language>/` directory with these files:
  - `mod.rs` - module declarations
  - `engine.rs` - implement `LanguageEngine` trait (copy and modify an existing engine)
  - `kinds.rs` - language-specific mutations (merged with common mutations)
  - `syntax.rs` - grammar node & field names from `grammars/<lang>/src/node-types.json`

## 4. Language registration

- Add your language module to `src/languages/mod.rs`
- Register the engine in `src/main.rs` by adding a `registry.register()` call

## 5. Tests and examples

- Add example files under `tests/<language>/examples/`
- Add tests under `tests/<language>/`

## 6. Validate

- `just check`
- `mewt print mutations --language <language>` shows your slugs
- `mewt print mutants --target tests/<language>/examples/...` generates mutants for example files
