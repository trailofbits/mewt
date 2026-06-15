// External scanner.
//
// Background: tree-sitter parsers can call hand-written C code for
// tokens the regex lexer can't handle (indentation, lookahead-driven
// decisions, etc.). That code is called the "external scanner". It
// exposes a fixed set of C entry points named
// `tree_sitter_<grammar>_external_scanner_{create,destroy,serialize,
// deserialize,scan}`. The parser invokes `scan` whenever it needs to
// resolve the next token, passing two things:
//
//   - a `TSLexer*` that lets the scanner read source characters
//     (via `lexer->lookahead`) and advance past them
//     (via `lexer->advance`).
//   - a `valid_symbols` array indicating which tokens the parser
//     would accept right now. The scanner reads it to decide what
//     to emit, or, in our case, to decide when to emit nothing.
//
// What this file does:
//   Almost everything here is the upstream tree-sitter-haskell
//   scanner. We wrap it so we can intercept one specific case
//   (multi-token choice return types like `ContractId Vault`).
//
// How the wrapping works:
//   1. Rename upstream's `tree_sitter_haskell_external_scanner_*`
//      entry points to `daml_inner_external_scanner_*` via #define.
//   2. `#include` upstream's scanner.c verbatim so it ends up
//      compiled under those renamed names.
//   3. Define our own `tree_sitter_daml_external_scanner_*` entry
//      points that delegate to the renamed upstream functions,
//      except for `scan`, which checks one thing first (see below)
//      before handing off.
//
// What `scan` intercepts:
//   At a choice's return-type slot (where `_daml_type_apply_guard`
//   is in `valid_symbols`), if the next non-space character starts
//   a constructor (uppercase, `(`, or `[`), we don't call upstream
//   at all. That stops upstream from opening a layout block right
//   after the first type word, which would otherwise truncate
//   `ContractId Vault` to a single token. The grammar then accepts
//   the second word as another type-apply argument.
//
// Build flag: tree-sitter's own runtime header `src/tree_sitter/array.h`
// (shipped by the tree-sitter generator into every grammar repo, not
// part of tree-sitter-haskell) has a latent strict-aliasing bug in
// `array_push`. Compile parser.c and this file with
// `-fno-strict-aliasing`. The `just build` recipe sets the flag.

#define tree_sitter_haskell_external_scanner_create      daml_inner_external_scanner_create
#define tree_sitter_haskell_external_scanner_destroy     daml_inner_external_scanner_destroy
#define tree_sitter_haskell_external_scanner_serialize   daml_inner_external_scanner_serialize
#define tree_sitter_haskell_external_scanner_deserialize daml_inner_external_scanner_deserialize
#define tree_sitter_haskell_external_scanner_scan        daml_inner_external_scanner_scan

#include "haskell-upstream/scanner.c"

// Indices into the `valid_symbols` array. Background: each external
// symbol declared in grammar.js gets a fixed position in the array.
// The exact order is in the enum
// `ts_external_scanner_symbol_identifiers` in the generated
// src/parser.c. The header below is regenerated from that enum by
// the `_gen-scanner-indices` just recipe (wired into `just regen`),
// so an upstream tree-sitter-haskell bump that reorders externals
// updates these constants automatically.
#include "scanner_indices.h"

void *tree_sitter_daml_external_scanner_create(void) {
  return daml_inner_external_scanner_create();
}

void tree_sitter_daml_external_scanner_destroy(void *payload) {
  daml_inner_external_scanner_destroy(payload);
}

unsigned tree_sitter_daml_external_scanner_serialize(void *payload, char *buf) {
  return daml_inner_external_scanner_serialize(payload, buf);
}

void tree_sitter_daml_external_scanner_deserialize(void *payload, const char *buf, unsigned length) {
  daml_inner_external_scanner_deserialize(payload, buf, length);
}

bool tree_sitter_daml_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  // Suppress the interior layout-start in choice return-type
  // positions when the next token starts with a constructor. This
  // keeps `choice X : ContractId Vault` from being chopped after
  // `ContractId`.
  //
  // All three conditions must hold:
  //   - we are at a slot that uses the type-apply guard,
  //   - upstream would otherwise open an interior layout here,
  //   - the next non-space character is uppercase, `(`, or `[`.
  //
  // The keywords that follow a choice return type in real DAML
  // (`with`, `controller`, `observer`, `where`, `do`, `nonconsuming`,
  // ...) all begin with a lowercase letter, so they pass straight
  // through to upstream unchanged.
  if (valid_symbols[DAML_SYM_TYPE_APPLY_GUARD]
      && valid_symbols[DAML_SYM_CMD_LAYOUT_START]) {
    int32_t c = lexer->lookahead;
    while (c == ' ' || c == '\t') {
      lexer->advance(lexer, true);
      c = lexer->lookahead;
    }
    if ((c >= 'A' && c <= 'Z') || c == '(' || c == '[') {
      return false;
    }
  }

  // Suppress upstream's "close enclosing layout on `=` or `,`" when
  // the parser is between fields of a record-`with` block. Without
  // this, `(Foo with f = x, g = y)` closes the with-block at the
  // first `f` (treating it as a punned field), then can't shift `=`.
  // Upstream encodes `=` and `,` as `LTexpCloser` (scanner.c
  // case '=' in lex_symop, case ',' in lex_brackets), which routes
  // through `token_end_layout_texp` and emits a layout-end whenever
  // any enclosing TExp context is on the stack (parens, brackets,
  // braces). The behaviour is correct for ordinary `do`/`let`
  // layouts but wrong for DAML record-with, where `=` is meaningful
  // inside a field_bind and `,` is the inter-field separator.
  //
  // The guard symbol is in `valid_symbols` exactly between a field
  // name and the optional `= <exp>` tail (see `field_bind` in
  // grammar/daml.js), so checking it here scopes the suppression to
  // the right parser state. We do NOT suppress `)` or `]`, which
  // legitimately close the enclosing paren/bracket and along with
  // it the inner with-block. Returning false here lets the regex
  // lexer emit the literal `=` or `,` token, which the parser
  // shifts to extend or advance the field_bind list.
  if (valid_symbols[DAML_SYM_IN_EXP_WITH]) {
    int32_t c = lexer->lookahead;
    while (c == ' ' || c == '\t') {
      lexer->advance(lexer, true);
      c = lexer->lookahead;
    }
    if (c == '=' || c == ',') {
      return false;
    }
  }

  return daml_inner_external_scanner_scan(payload, lexer, valid_symbols);
}
