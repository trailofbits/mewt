use std::sync::OnceLock;
use tree_sitter::{Language as TsLanguage, Node};

use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, PartialMutant, Target};
use crate::utils::{
    calculate_line_offset, is_in_comment, node_text, parse_source, visit_nodes_with_cursor,
};

use super::mutations::DAML_MUTATIONS;
use super::syntax::{fields, nodes};

// DAML is a strict superset of Haskell, so we reuse the tree-sitter-haskell
// grammar (vendored at grammars/haskell/) for parsing. DAML-specific
// constructs that the grammar does not understand surface as ERROR-recovered
// subtrees; the leaf kinds we mutate on (conditional / infix / constructor)
// remain intact inside that recovery.
static DAML_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_haskell() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct DamlLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for DamlLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Common slugs DAML does not emit, with the reason. `new()` filters these out so
// `print mutations` only lists mutations that actually fire. A slug is here
// either because the construct doesn't exist in Haskell/DAML, or because it would
// need a custom pass over the ERROR-recovered tree that isn't implemented yet.
const UNSUPPORTED_COMMON: &[(&str, &str)] = &[
    // Not applicable: no such construct in the language.
    ("WF", "no `while` loops; DAML iterates via recursion"),
    ("LC", "no `break` / `continue` statements"),
    ("AAOS", "no mutable compound assignment (`+=` etc.)"),
    ("BAOS", "no compound bitwise assignment"),
    (
        "BOS",
        "bitwise ops are Data.Bits functions (`.&.`, `.|.`, `xor`), not `& | ^` tokens",
    ),
    (
        "SOS",
        "`>>` is monadic sequencing, not a shift; real shifts are `shiftL` / `shiftR`",
    ),
    (
        "SAOS",
        "`>>=` is monadic bind; no compound shift assignment",
    ),
    // Deferred: feasible but needs a custom Haskell-shaped pass.
    ("NR", "DAML negation is `not x` (a function), not `!x`"),
    ("AS", "curried application has no comma-separated arg list"),
    (
        "ER",
        "replacing an expression with `error \"mewt\"` needs a custom pass",
    ),
    ("CR", "commenting out a binding needs a custom pass"),
];

fn is_unsupported_common(slug: &str) -> bool {
    UNSUPPORTED_COMMON.iter().any(|(s, _)| *s == slug)
}

impl DamlLanguageEngine {
    pub fn new() -> Self {
        // Expose only the common mutations DAML actually emits, so
        // `print mutations --language DAML` never advertises a no-op. The
        // deliberate exclusions and their reasons live in UNSUPPORTED_COMMON.
        let mut mutations: Vec<Mutation> = COMMON_MUTATIONS
            .iter()
            .filter(|m| !is_unsupported_common(m.slug))
            .cloned()
            .collect();
        mutations.extend_from_slice(DAML_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for DamlLanguageEngine {
    fn name(&self) -> &'static str {
        "DAML"
    }

    fn extensions(&self) -> &[&'static str] {
        &["daml"]
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        let language =
            DAML_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_haskell()) });

        let tree = match parse_source(source, language) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let root = tree.root_node();

        let mut all_mutants = Vec::new();
        for m in &self.mutations {
            match m.slug {
                "IF" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::CONDITIONAL,
                        fields::IF,
                        &["if"],
                        "False",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "IF")),
                ),
                "IT" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::CONDITIONAL,
                        fields::IF,
                        &["if"],
                        "True",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "IT")),
                ),
                "BL" => all_mutants.extend(
                    boolean_literal_swaps(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "BL")),
                ),
                "AOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::INFIX],
                        &["+", "-", "*", "/"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AOS")),
                ),
                "COS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::INFIX],
                        // Haskell/DAML uses /= for inequality, not !=.
                        &["==", "/=", "<", "<=", ">", ">="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "COS")),
                ),
                "LOS" => all_mutants.extend(
                    patterns::shuffle_operators(root, source, &[nodes::INFIX], &["&&", "||"])
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "LOS")),
                ),
                "CPS" => all_mutants.extend(
                    controller_party_swaps(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "CPS")),
                ),
                "CPR" => all_mutants.extend(
                    controller_party_removals(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "CPR")),
                ),
                // Only the slugs above are exposed by `new()`. Anything else
                // reaching this loop means a slug was added to the engine's
                // mutation list without a matching arm; fail loudly rather
                // than silently emit nothing. Deliberate non-emitters are
                // filtered out up front via UNSUPPORTED_COMMON.
                other => panic!("Unhandled mutation slug in DAML engine: {other}"),
            }
        }
        all_mutants
    }
}

// Boolean literal flip. In Haskell, `data Bool = True | False`, so the
// booleans aren't keywords; they're ordinary data constructors that share
// the parse-tree kind `constructor` with every other constructor. For
// example, all three of these are kind `constructor`:
//
//   constructor  "True"       the boolean literal we want to flip
//   constructor  "Just"       a standard-library constructor (the `Just`
//                             case of `Maybe a = Nothing | Just a`)
//   constructor  "TrueColor"  a user-defined constructor
//
// So matching on kind alone is not enough: it would also rewrite unrelated
// constructors like `Just`. Worse, the shared patterns::shuffle_nodes
// helper matches by *substring*, so it would turn `TrueColor` into
// `FalseColor`. We match on the exact text `True`/`False` instead.
fn boolean_literal_swaps(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() != nodes::CONSTRUCTOR || is_in_comment(&node) {
            return;
        }
        let text = node_text(&node, source);
        let new_text = match text {
            "True" => "False",
            "False" => "True",
            _ => return,
        };

        mutants.push(PartialMutant {
            byte_offset: node.start_byte() as u32,
            line_offset: calculate_line_offset(source, node.start_byte()),
            old_text: text.to_string(),
            new_text: new_text.to_string(),
        });
    });
    mutants
}

// Controller authorisation mutations: CPS (swap a party for another in
// the same template) and CPR (drop one party from a multi-party list).
//
// Both aim to emit only mutants that compile.
//
// Tree-sitter-haskell has no `template_definition` or `choice_definition`
// node, so we can't structurally ask "what's in this template's `with`
// block?". The grammar wraps the DAML-specific region in an ERROR node
// and treats `template` / `with` / `choice` / `controller` as ordinary
// curried function application, e.g. `template T with owner` parses as
// the left-associative `((template T) with) owner`. The tree shape:
//
//   ERROR
//     apply                                     // "template T with owner"
//       apply
//         apply
//           variable             "template"
//           constructor          "T"
//         variable               "with"
//       variable                 "owner"
//     constructor_operator       ":"
//     infix                                     // ...rest of template body
//
// Subtree structure inside ERROR is unreliable, but each leaf still has
// the right kind at the right byte range. We collect `variable`,
// `constructor`, and `constructor_operator` leaves in source order and
// recognise the patterns we need (`name : Party` declarations, template
// boundaries, `controller <party-list>` sites) by byte adjacency.

#[derive(Clone, Copy)]
struct Leaf<'a> {
    kind: &'static str,
    text: &'a str,
    start: usize,
    end: usize,
}

struct PartyDecl<'a> {
    byte_offset: usize,
    name: &'a str,
}

struct TemplateScope<'a> {
    /// Half-open byte range of the template, from its `template` keyword
    /// to the next `template` keyword or the end of the file.
    range: (usize, usize),
    /// Party-typed parameters declared in the template's `with` block,
    /// i.e. before the first `choice` keyword inside the template's
    /// range. These are the cross-choice swap candidates: they are in
    /// scope at every `controller` site in the template.
    ///
    /// Choice-local Party parameters (declared in a choice's own `with`
    /// block) are not stored here. We add them per-site in
    /// `controller_party_swaps`, so they only affect their own choice's
    /// controller and stay out of scope in other choices.
    party_params: Vec<&'a str>,
}

fn controller_party_swaps(root: Node, source: &str) -> Vec<PartialMutant> {
    let leaves = collect_cps_leaves(root, source);
    let templates = collect_template_scopes(&leaves, source);
    let party_decls = find_party_declarations(&leaves, source);

    let mut mutants = Vec::new();
    for (i, leaf) in leaves.iter().enumerate() {
        if !is_controller_keyword(leaf) {
            continue;
        }
        let parties = parse_controller_list(i, &leaves, source);
        if parties.is_empty() {
            continue;
        }
        let Some(template) = enclosing_template(&templates, leaf.start) else {
            continue;
        };
        // Start from the template-level params (in scope at every choice),
        // then add any Party params declared in this choice's own `with`
        // block: those declared between the enclosing `choice` keyword and
        // this `controller` site. Because the window is bounded by THIS
        // controller's enclosing choice, a different choice's controller
        // sees a different window and never the other choice's locals.
        let mut candidates: Vec<&str> = template.party_params.clone();
        if let Some(choice_start) = enclosing_choice_start(&leaves, i, template.range.0) {
            for decl in &party_decls {
                // `leaf.start` is this `controller` keyword: collect only the
                // Party decls written between the enclosing `choice` and this
                // controller, i.e. this choice's own with-block params.
                if decl.byte_offset > choice_start && decl.byte_offset < leaf.start {
                    candidates.push(decl.name);
                }
            }
        }
        // De-duplicate while preserving order (a template-level name may
        // also appear as a choice-local decl; offer it only once).
        let mut deduped: Vec<&str> = Vec::new();
        for name in candidates {
            if !deduped.contains(&name) {
                deduped.push(name);
            }
        }
        let alternatives = swap_alternatives(&deduped, &parties);
        for party in &parties {
            for alt in &alternatives {
                mutants.push(PartialMutant {
                    byte_offset: party.start as u32,
                    line_offset: calculate_line_offset(source, party.start),
                    old_text: party.text.to_string(),
                    new_text: alt.to_string(),
                });
            }
        }
    }
    mutants
}

fn controller_party_removals(root: Node, source: &str) -> Vec<PartialMutant> {
    let leaves = collect_cps_leaves(root, source);

    let mut mutants = Vec::new();
    for (i, leaf) in leaves.iter().enumerate() {
        if !is_controller_keyword(leaf) {
            continue;
        }
        let parties = parse_controller_list(i, &leaves, source);
        // Dropping the only party from a single-party `controller p`
        // leaves the choice with no controller, which doesn't compile.
        if parties.len() < 2 {
            continue;
        }
        for idx in 0..parties.len() {
            // Skip removing a party when an identical name still remains in the
            // list: dropping a duplicate authoriser (`controller a, a`) does not
            // shrink the required authorization set, so the mutant is a
            // guaranteed no-op (and duplicates of one another). Only offer a
            // removal that actually changes the set of authorisers.
            let removed = parties[idx].text;
            let duplicate_remains = parties
                .iter()
                .enumerate()
                .any(|(j, p)| j != idx && p.text == removed);
            if duplicate_remains {
                continue;
            }
            let (start, end) = removal_byte_range(&parties, idx);
            mutants.push(PartialMutant {
                byte_offset: start as u32,
                line_offset: calculate_line_offset(source, start),
                old_text: source[start..end].to_string(),
                new_text: String::new(),
            });
        }
    }
    mutants
}

fn is_controller_keyword(leaf: &Leaf) -> bool {
    leaf.kind == nodes::VARIABLE && leaf.text == "controller"
}

/// Find the template scope whose byte range contains `byte_offset`.
fn enclosing_template<'a, 's>(
    templates: &'a [TemplateScope<'s>],
    byte_offset: usize,
) -> Option<&'a TemplateScope<'s>> {
    templates
        .iter()
        .find(|t| byte_offset >= t.range.0 && byte_offset < t.range.1)
}

/// Template Party parameters that are *not* already in the controller's
/// list. Excluding them prevents duplicate-looking output like
/// `controller primary, counter` mutating to `controller counter, counter`
/// and guarantees every swap changes the *set* of required authorisers.
fn swap_alternatives<'a>(template_parties: &[&'a str], in_list: &[Leaf<'a>]) -> Vec<&'a str> {
    let mut alternatives = Vec::new();
    for &candidate in template_parties {
        let already_in_list = in_list.iter().any(|p| p.text == candidate);
        if !already_in_list {
            alternatives.push(candidate);
        }
    }
    alternatives
}

/// Byte range to delete to remove `parties[idx]` from a multi-party
/// controller list. The range includes the adjacent comma+whitespace so
/// the resulting `controller` clause is well-formed.
///
/// The first party owns the separator on its right (`a, b` becomes `b`);
/// every later party owns the separator on its left (`a, b` becomes `a`).
fn removal_byte_range(parties: &[Leaf], idx: usize) -> (usize, usize) {
    if idx == 0 {
        (parties[0].start, parties[1].start)
    } else {
        (parties[idx - 1].end, parties[idx].end)
    }
}

/// Walk leaves forward from `controller_idx` (the position of the
/// `controller` keyword) and return the party variables in its
/// comma-separated list. Returns an empty vector if the controller's
/// expression is more complex than a plain party list (parenthesised,
/// record access, function application, qualified name, ...) - we'd rather
/// emit zero mutants for an unusual controller than emit one that doesn't
/// compile.
///
/// This cannot over-read into the next choice: only a comma gap continues
/// the walk, and no legal DAML construct following a party list begins with
/// a bare comma, so the list is implicitly bounded.
///
/// TODO: missed coverage on `controller (alice)`. A bare party wrapped in
/// superfluous parens would be safe to swap (the result `controller
/// (custodian)` compiles), but we currently abort the site because we
/// can't tell from a single `(` whether the contents are a plain variable
/// or something more involved (`controller (alice.delegate)`,
/// `controller (someFunc alice)`, ...). To lift: when the gap before the
/// first party is exactly `(` and the gap after the variable is exactly
/// `)`, treat it as a plain-party site after all. Anything else inside
/// the parens (operators, dots, more variables) should keep aborting.
fn parse_controller_list<'a>(
    controller_idx: usize,
    leaves: &[Leaf<'a>],
    source: &'a str,
) -> Vec<Leaf<'a>> {
    let mut parties: Vec<Leaf<'a>> = Vec::new();
    let mut last_end = leaves[controller_idx].end;

    // `controller_idx` is the `controller` keyword leaf itself, so `+1`
    // starts the scan at the first leaf after the keyword (the first
    // candidate party); without it the loop would inspect the keyword token.
    for leaf in &leaves[controller_idx + 1..] {
        let gap_text = &source[last_end..leaf.start];
        let gap = classify_gap(gap_text);

        if parties.is_empty() {
            // The first party follows the `controller` keyword across
            // whitespace. Anything else means the controller is not a plain
            // party list, so we stop before collecting one.
            match gap {
                Gap::Whitespace if leaf.kind == nodes::VARIABLE => {
                    parties.push(*leaf);
                    last_end = leaf.end;
                }
                _ => break,
            }
        } else {
            match gap {
                // A subsequent party is separated from the previous one by a
                // single comma.
                Gap::Comma if leaf.kind == nodes::VARIABLE => {
                    parties.push(*leaf);
                    last_end = leaf.end;
                }
                // A bare token following a party on the SAME line with no
                // comma (whitespace-only gap, no newline) means the controller
                // is a function application or other complex expression, not a
                // party list - e.g. `controller resolveActor owner` is
                // `controller (resolveActor owner)`. We emit nothing rather
                // than mistaking the function name for a party and producing a
                // mutant that doesn't compile.
                Gap::Whitespace if !gap_text.contains('\n') => return Vec::new(),
                // Structural punctuation (`.`, `(`, `:`, ...) after at least
                // one party means the controller isn't a plain party list.
                Gap::Punctuation => return Vec::new(),
                // Anything else (a newline before the next token, or a
                // trailing keyword like `do` / `where`) ends the list cleanly,
                // keeping the parties already collected.
                _ => break,
            }
        }
    }
    parties
}

enum Gap {
    /// Whitespace only.
    Whitespace,
    /// A single comma (whitespace stripped).
    Comma,
    /// Contains a non-identifier character: `.`, `(`, `:`, ... Treated as
    /// "the controller expression is more complex than a plain party
    /// list."
    Punctuation,
    /// Anything else (for example a trailing keyword like `do` or `where`);
    /// ends the list.
    Other,
}

fn classify_gap(text: &str) -> Gap {
    let non_ws: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if non_ws.is_empty() {
        return Gap::Whitespace;
    }
    if non_ws == "," {
        return Gap::Comma;
    }
    let has_punctuation = non_ws.chars().any(|c| !c.is_alphanumeric() && c != '_');
    if has_punctuation {
        Gap::Punctuation
    } else {
        Gap::Other
    }
}

fn collect_cps_leaves<'a>(root: Node, source: &'a str) -> Vec<Leaf<'a>> {
    let mut leaves: Vec<Leaf<'a>> = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        let kind = node.kind();
        let is_interesting = matches!(
            kind,
            nodes::VARIABLE | nodes::CONSTRUCTOR | nodes::CONSTRUCTOR_OPERATOR
        );
        if !is_interesting {
            return;
        }
        let start = node.start_byte();
        let end = node.end_byte();
        if start < end {
            leaves.push(Leaf {
                kind,
                text: &source[start..end],
                start,
                end,
            });
        }
    });
    leaves.sort_by_key(|l| l.start);
    leaves
}

fn collect_template_scopes<'a>(leaves: &[Leaf<'a>], source: &str) -> Vec<TemplateScope<'a>> {
    let template_starts: Vec<usize> = leaves
        .iter()
        .filter(|l| l.kind == nodes::VARIABLE && l.text == "template")
        .map(|l| l.start)
        .collect();
    let party_decls = find_party_declarations(leaves, source);

    let mut scopes = Vec::new();
    for (i, &template_start) in template_starts.iter().enumerate() {
        // Templates are top-level and don't nest, so each one runs until the
        // next `template` keyword; the last runs to the end of the source.
        let template_end = template_starts.get(i + 1).copied().unwrap_or(source.len());
        // The template's `with` block ends at its first `choice` keyword;
        // anything after that belongs to choice-local with-blocks, which
        // we deliberately exclude (see TemplateScope::party_params).
        let with_block_end =
            first_choice_in(leaves, template_start, template_end).unwrap_or(template_end);
        let party_params = party_decls
            .iter()
            .filter(|d| d.byte_offset >= template_start && d.byte_offset < with_block_end)
            .map(|d| d.name)
            .collect();
        scopes.push(TemplateScope {
            range: (template_start, template_end),
            party_params,
        });
    }
    scopes
}

fn first_choice_in(leaves: &[Leaf], range_start: usize, range_end: usize) -> Option<usize> {
    // A template field named `choice` is legal DAML (`choice` is a soft
    // keyword), so only treat `choice` as the keyword when the next leaf is
    // the choice name (a constructor), not a `:` type annotation.
    leaves.iter().enumerate().find_map(|(i, l)| {
        let is_choice_keyword = l.kind == nodes::VARIABLE
            && l.text == "choice"
            && l.start >= range_start
            && l.start < range_end
            && leaves
                .get(i + 1)
                .is_some_and(|next| next.kind == nodes::CONSTRUCTOR);
        is_choice_keyword.then_some(l.start)
    })
}

/// Start byte of the nearest `choice` keyword preceding the `controller` at
/// `controller_idx`, within the enclosing template (keyword at or after
/// `template_start` and strictly before the controller leaf). Returns `None`
/// when the controller is not inside a choice we can see.
///
/// We use the same soft-keyword guard as `first_choice_in`: a `choice` leaf
/// only counts when it is a `variable` whose text is `choice` and the next
/// leaf is the choice name (a constructor), so a template field named
/// `choice` does not match.
fn enclosing_choice_start(
    leaves: &[Leaf],
    controller_idx: usize,
    template_start: usize,
) -> Option<usize> {
    let controller_start = leaves[controller_idx].start;
    leaves
        .iter()
        .enumerate()
        .filter(|(i, l)| {
            l.kind == nodes::VARIABLE
                && l.text == "choice"
                && l.start >= template_start
                && l.start < controller_start
                && leaves
                    .get(i + 1)
                    .is_some_and(|next| next.kind == nodes::CONSTRUCTOR)
        })
        .map(|(_, l)| l.start)
        .max()
}

fn find_party_declarations<'a>(leaves: &[Leaf<'a>], source: &str) -> Vec<PartyDecl<'a>> {
    let mut decls = Vec::new();
    for window in leaves.windows(3) {
        let name_leaf = &window[0];
        let colon_leaf = &window[1];
        let type_leaf = &window[2];
        if is_party_declaration(name_leaf, colon_leaf, type_leaf, source) {
            decls.push(PartyDecl {
                byte_offset: name_leaf.start,
                name: name_leaf.text,
            });
        }
    }
    decls
}

/// Three adjacent leaves form a Party parameter declaration when they
/// match the shape `<name> : Party` exactly: a variable, then a
/// constructor-operator that's a single colon, then a constructor whose
/// text is `Party`, all separated by whitespace only.
///
/// `Party` must be the COMPLETE type. A binding whose type merely starts with
/// `Party` and continues on the same line (e.g. `notify : Party -> ()`) is a
/// function, not a party, and is rejected: offering it as a controller swap
/// target would emit a non-compiling mutant. The function arrow `->` is an
/// `operator` leaf, which `collect_cps_leaves` does not collect, so we can't
/// peek it as a typed leaf; instead we inspect the raw source from the end of
/// `Party` up to the next newline. Any non-whitespace there (the `->`, a type
/// argument, ...) means the type continues. Crossing the newline first is the
/// legitimate next-field case (`owner : Party` newline `bob : Party`).
fn is_party_declaration(name: &Leaf, colon: &Leaf, ty: &Leaf, source: &str) -> bool {
    let is_name = name.kind == nodes::VARIABLE;
    let is_colon = colon.kind == nodes::CONSTRUCTOR_OPERATOR && colon.text == ":";
    let is_party = ty.kind == nodes::CONSTRUCTOR && ty.text == "Party";
    let only_whitespace_between = source[name.end..colon.start]
        .chars()
        .all(char::is_whitespace)
        && source[colon.end..ty.start].chars().all(char::is_whitespace);
    let type_continues_on_same_line = source[ty.end..]
        .chars()
        .take_while(|&c| c != '\n')
        .any(|c| !c.is_whitespace());
    is_name && is_colon && is_party && only_whitespace_between && !type_continues_on_same_line
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = DamlLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in DAML engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text: &str = "module Smoke where\n\nfoo : Int\nfoo = 1\n";
        let target = Target {
            id: 0,
            path: PathBuf::from("smoke.daml"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "DAML".to_string(),
        };
        let engine = DamlLanguageEngine::new();
        let _ = engine.mutate(&target);
    }

    // Every common mutation is either emitted by DAML or explicitly listed in
    // UNSUPPORTED_COMMON with a reason. This is the safety net for the choice
    // to advertise only implemented slugs: when a new COMMON_MUTATION lands
    // upstream, this test fails until someone decides to implement it or
    // document why DAML skips it.
    #[test]
    fn unsupported_common_slugs_are_exhaustive() {
        let exposed: HashSet<&str> = DamlLanguageEngine::new()
            .get_mutations()
            .iter()
            .map(|m| m.slug)
            .collect();

        let unhandled: Vec<&str> = COMMON_MUTATIONS
            .iter()
            .map(|m| m.slug)
            .filter(|slug| !exposed.contains(slug) && !is_unsupported_common(slug))
            .collect();

        assert!(
            unhandled.is_empty(),
            "common mutation slugs are neither emitted nor documented as \
             unsupported: {unhandled:?}. Implement them in DAML or add them to \
             UNSUPPORTED_COMMON with a reason."
        );
    }

    // Guards against a stale skip list: a slug DAML actually emits must not
    // also claim to be unsupported.
    #[test]
    fn unsupported_list_does_not_overlap_emitted_slugs() {
        let exposed: HashSet<&str> = DamlLanguageEngine::new()
            .get_mutations()
            .iter()
            .map(|m| m.slug)
            .collect();
        let overlap: Vec<&str> = UNSUPPORTED_COMMON
            .iter()
            .map(|(slug, _)| *slug)
            .filter(|slug| exposed.contains(slug))
            .collect();
        assert!(
            overlap.is_empty(),
            "slugs listed as unsupported but also emitted: {overlap:?}"
        );
    }
}
