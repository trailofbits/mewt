// DAML grammar. Builds on tree-sitter-haskell because DAML is Haskell
// plus a handful of contract-specific keywords (template, choice, key,
// signatory, ...). We inherit the Haskell-shaped half from upstream
// and only override what DAML changes.
//
// Surface differences from Haskell:
//
//   1. `:` and `::` are swapped.
//
//        Haskell:   x :: Int             xs = 1 : 2 : []
//        DAML:      x : Int              xs = 1 :: 2 :: []
//
//   2. New top-level and in-template forms:
//
//        template Vault with                interface Token where
//          owner: Party                       viewtype TokenView
//        where                                choice Burn : () ...
//          signatory owner
//          choice Withdraw : () ...
//
// What this file does (overrides on top of tree-sitter-haskell):
//
//   - Adds `template` and `interface` to the set of valid top-level
//     declarations. Adds `record_with` to expressions (DAML's
//     `e with f1 = v, f2, ..`). Rule bodies live in grammar/daml.js.
//
//   - Flips `:` and `::` by overriding five upstream rules. See the
//     `:` / `::` swap block below.
//
//   - Adds the `with`-block form of a data constructor
//     (`data X = X with f : T`). Disables upstream's "infix data
//     constructor" branch because its scanner lookahead misreads
//     DAML's `with` as infix syntax.
//
//   - Declares two phantom external tokens used by src/scanner.c
//     to keep multi-token choice return types (`ContractId Vault`)
//     and parenthesised record-with field lists (`(Foo with f = x,
//     g = y)`) from being broken up by layout.

const haskell = require('tree-sitter-haskell/grammar.js')
const daml = require('./grammar/daml.js')
const { sep1, braces } = require('tree-sitter-haskell/grammar/util.js')

module.exports = grammar(haskell, {
  name: 'daml',

  // Phantom externals declared here. The pattern: the grammar marks
  // a parse position with each name; the scanner sees the marker in
  // its `valid_symbols` list at lex time and uses it as a flag to
  // decide what to do at that point (open or suppress a layout, etc).
  // The scanner never emits these tokens itself. See src/scanner.c
  // for the scanner-side handling.
  externals: ($, previous) => [
    ...previous,
    // Marks a choice's return-type slot. Without the phantom, the
    // scanner opens a layout block after the first type word, so:
    //
    //   choice Mint : ContractId Vault
    //
    // parses with `ContractId` as the whole return type and `Vault`
    // drops into ERROR. The phantom suppresses the layout-open so
    // `ContractId Vault` stays as one return type.
    $._daml_type_apply_guard,

    // Marks positions inside a record-`with` field list (between
    // fields). Without the phantom, upstream's rule "`=` and `,`
    // close the enclosing layout when inside parens" fires, so:
    //
    //   (Foo with f = x, g = y)
    //
    // closes the with-block at the first `=` and the rest of the
    // line drops into ERROR. The phantom suppresses that close so
    // the full with-block parses.
    $._daml_in_exp_with,
  ],

  // tree-sitter normally wants a single parse for each input.
  // Declaring a conflict here says "this input has two valid
  // readings; try both and keep whichever fits the rest of the
  // code." Used when the right reading depends on context the
  // parser hasn't seen yet.
  conflicts: ($, previous) => [
    ...previous,
    // The text `pattern Foo :: Int` has two valid readings:
    //
    //   1. A `pattern` declaration with a type signature.
    //   2. A constructor pattern (`Foo`) followed by `::` as a
    //      list-cons operator and another pattern.
    //
    // Reading 2 only exists because the `:` / `::` swap below
    // makes `::` a normal constructor operator. Both are legal
    // Haskell, so we keep both alive and let context decide.
    [$.pattern, $._patsyn_signature],
  ],

  rules: {
    ...daml,

    // Top-level decls: anything upstream allows, plus DAML's
    // `template` and `interface` forms.
    declaration: ($, previous) => choice(previous, $.template, $.interface),

    // ----- the `:` / `::` swap -------------------------------------
    //
    // Background: Haskell splits operators into two families.
    //
    //   varsym   any operator NOT starting with ':'   (`+`, `==`, `>>=`)
    //   consym   any operator starting with ':'       (`:`, `::`, `:+:`)
    //
    // tree-sitter-haskell's scanner emits the matching token for
    // either family; the grammar rule `_sym` accepts either. In
    // upstream Haskell, `:` is cons and lives in the consym family,
    // so the scanner claims `:` wherever it appears as an operator.
    //
    // In DAML, `:` is NOT an operator. It's a type-annotation
    // separator at known syntactic positions (signatures, fields,
    // key types). To make the scanner stop claiming `:` for the
    // consym role, we drop the consym alternative from `_sym`.
    // After the override, the regular lexer is free to emit `:` as
    // a plain literal token wherever the grammar asks for it.
    //
    // Five overrides total, with one example DAML line each:
    //
    //   constructor_operator   xs = 1 :: 2 :: []             (cons, the new `::`)
    //   _sym                   x : Int                       (frees `:` at sig positions)
    //   _type_annotation       f :: Int -> a where a : *     (annotation in type body)
    //   _kind_annotation       data T (a : *) = ...          (kind annotation)
    //   field                  template V with owner: Party  (with-block field)
    constructor_operator: _ => '::',
    _sym: $ => choice($._operator_alias, $._constructor_operator_alias),
    _type_annotation: $ => seq(':', field('type', $.quantified_type)),
    _kind_annotation: $ => seq(':', field('kind', $.quantified_type)),
    field: ($, _previous) => prec('annotated', seq(
      sep1(',', field('name', $.field_name)),
      ':',
      field('type', $._parameter_type),
    )),


    // Record-style data constructor: `data X = X with f1: T1; f2: T2`.
    // Three forms exist in the language as a whole:
    //
    //   Haskell braces:   data X = X { f1 :: T1, f2 :: T2 }   (upstream)
    //   Positional:       data X = X Int Bool                 (upstream)
    //   DAML with-block:  data X = X with f1: T1; f2: T2      (added here)
    _datacon_record: ($, previous) => choice(
      previous,
      seq(
        field('name', $._constructor),
        field('fields', $.with_fields),
      ),
    ),

    // Brace-form record fields. DAML accepts either `,` or `;` as the
    // field separator, and the two can be mixed in one block:
    //
    //   data Foo = Foo { x : Int, y : Int; z : Int }
    //
    // Upstream Haskell only accepts `,`. The override widens the
    // separator and keeps the trailing-separator allowance. Shared
    // across data, GADT, and newtype record constructors.
    _record_fields: $ => braces($,
      optional(seq(
        field('field', $.field),
        repeat(seq(choice(',', ';'), field('field', $.field))),
        optional(choice(',', ';')),
      )),
    ),

    // Upstream's "infix data constructor" branch (`a :+: b` inside
    // a data declaration) triggers a scanner lookahead that misreads
    // DAML's `data X = X with ...` as infix syntax. We disable the
    // branch by binding it to a never-matching token: `[^\s\S]` is
    // an empty character class the lexer cannot satisfy. DAML has
    // no user-defined infix data constructors, so nothing of value
    // is lost.
    _datacon_infix: $ => token(prec(-1, /[^\s\S]/)),

    // Record-`with` syntax shows up in two roles. The same shape
    // `X with f1, f2 = v, ..` can either BUILD a record (an
    // expression: think "make this value") or DESTRUCTURE one (a
    // pattern: think "match against this shape and bind the fields").
    // Which role it plays depends on where it sits in the source.
    // We add it to both rule choices below, exposed as distinct
    // public nodes so consumers can tell them apart.

    // Expression form. Used on the right-hand side of `=`, inside
    // `do` blocks, anywhere a value is being computed:
    //
    //   create Vault with owner = alice, balance = 100
    //   let v = base with balance = 200
    //
    // Body lives in grammar/daml.js as `_exp_with`; public node
    // name is `record_with`.
    expression: ($, previous) => choice(
      previous,
      alias($._exp_with, $.record_with),
    ),

    // Pattern form. Used in function-head arguments and `case` arms,
    // anywhere a value is being taken apart:
    //
    //   addOwner (Vault with owner, ..) = ...
    //   case v of (Vault with owner) -> ...
    //
    // Body lives in grammar/daml.js as `_pat_with`; public node
    // name is `record_with_pat`. Kept as a distinct node from the
    // expression form so consumers can route on kind alone (no
    // parent-context check needed).
    pattern: ($, previous) => choice(
      previous,
      alias($._pat_with, $.record_with_pat),
    ),
  },
})
