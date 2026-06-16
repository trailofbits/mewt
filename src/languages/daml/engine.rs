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

static DAML_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_daml() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct DamlLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for DamlLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Common slugs DAML does not emit, with the reason. `new()` filters these out
// so `print mutations` only lists mutations that actually fire. A slug is
// here either because the construct doesn't exist in Haskell/DAML or because
// emitting it needs a dedicated AST traversal we have not written yet.
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
    (
        "NR",
        "DAML negation is `not x` (a function); no `!x` prefix token to strip",
    ),
    // Deferred: feasible against the grammar but needs a dedicated traversal we
    // have not written yet.
    ("AS", "curried application has no comma-separated arg list"),
    (
        "ER",
        "replacing an expression with `error \"mewt\"` needs a dedicated traversal",
    ),
    ("CR", "commenting out a binding needs a dedicated traversal"),
];

fn is_unsupported_common(slug: &str) -> bool {
    UNSUPPORTED_COMMON.iter().any(|(s, _)| *s == slug)
}

impl DamlLanguageEngine {
    pub fn new() -> Self {
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
            DAML_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_daml()) });

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
                // Only the slugs above are exposed by `new()`; reaching here
                // means a slug was added to the mutation list without a
                // matching arm. Fail loudly rather than emit nothing.
                other => panic!("Unhandled mutation slug in DAML engine: {other}"),
            }
        }
        all_mutants
    }
}

// Boolean literal flip. In Haskell, `data Bool = True | False`, so booleans
// aren't keywords; they're ordinary data constructors that share the
// parse-tree kind `constructor` with every other constructor. We match on
// the exact text `True` / `False`, never substring-replace.
// Tree-sitter model: `Node::kind()` returns the grammar rule name;
// `child_by_field_name` follows a named slot from `node-types.json`; byte offsets
// `start_byte`/`end_byte` index `source` directly; a `TreeCursor` is a reusable walker.
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

// Example: `controller alice` in a template with `bob : Party` becomes `controller bob`.
fn controller_party_swaps(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    for site in controller_sites(&root) {
        let parties = bare_party_variables(&site.controller);
        if parties.is_empty() {
            continue;
        }
        let candidates = swap_candidates(&site, &parties, source);
        for party in &parties {
            for cand in &candidates {
                mutants.push(PartialMutant {
                    byte_offset: party.start_byte() as u32,
                    line_offset: calculate_line_offset(source, party.start_byte()),
                    old_text: node_text(party, source).to_string(),
                    new_text: cand.clone(),
                });
            }
        }
    }
    mutants
}

// Example: `controller primary, counter` yields two mutants dropping each party in turn.
fn controller_party_removals(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    for site in controller_sites(&root) {
        let parties = bare_party_variables(&site.controller);
        // The grammar's `controller_decl` requires at least one party child,
        // so a single-party `controller p` has no removal we can emit
        // without producing an empty list that won't re-parse.
        if parties.len() < 2 {
            continue;
        }
        for idx in 0..parties.len() {
            let removed_text = node_text(&parties[idx], source);
            // Skip when an identical name still remains: dropping a duplicate
            // does not change the required authorization set, so the mutant
            // is a guaranteed no-op.
            let duplicate_remains = parties
                .iter()
                .enumerate()
                .any(|(j, p)| j != idx && node_text(p, source) == removed_text);
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

/// One `controller_decl` plus its enclosing `choice_decl` and `template`.
/// Holding all three lets us look up swap candidates without re-walking:
/// the template's `with_fields` gives template-level Party fields, the
/// choice's own `with_fields` (when present) gives choice-local ones.
// DAML: a `template` is a contract type; a `choice` is an action on it whose
// `controller` lists the Party values authorized to invoke it; `with_fields` is
// the `with`-block of typed parameters (Party, Int, ...).
struct ControllerSite<'a> {
    controller: Node<'a>,
    choice: Node<'a>,
    template: Node<'a>,
}

fn controller_sites<'a>(root: &Node<'a>) -> Vec<ControllerSite<'a>> {
    let mut sites = Vec::new();
    collect_controller_sites(*root, &mut sites);
    sites
}

/// Walks the AST top-down (visit a node, then recurse into its children) and
/// gathers every `controller_decl` along with its enclosing choice and
/// template. Hand-rolled rather than delegated to the shared visitor because
/// the visitor's callback hands out nodes with a closure-local lifetime,
/// which can't escape into the returned vector.
fn collect_controller_sites<'a>(node: Node<'a>, sites: &mut Vec<ControllerSite<'a>>) {
    if node.kind() == nodes::CONTROLLER_DECL && !is_in_comment(&node) {
        if let (Some(choice), Some(template)) = (
            ancestor_of_kind(&node, nodes::CHOICE_DECL),
            ancestor_of_kind(&node, nodes::TEMPLATE),
        ) {
            sites.push(ControllerSite {
                controller: node,
                choice,
                template,
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_controller_sites(child, sites);
    }
}

/// The bare identifiers a `controller_decl` lists as parties. Returns the
/// `variable` nodes themselves so callers can use their byte ranges and
/// text directly.
///
/// A `controller_decl`'s `party` field is a multi-field; each child is an
/// expression. We only collect children that are plain `variable` nodes.
/// Anything else (`parens (a)`, `apply f x`, `qualified A.B`, `infix`, ...)
/// drops the whole site rather than emit a partial list, because swapping
/// or removing only some children would rewrite a non-trivial expression.
/// Punctuation (`,`) is not a child of the `party` field, so there is no
/// byte-gap classification to do here.
///
/// Known limitation: production DAML often uses `(view this).field`,
/// `qualified M.party`, or `apply getController this` in `controller`; those
/// sites currently produce zero mutants.
fn bare_party_variables<'a>(controller: &Node<'a>) -> Vec<Node<'a>> {
    let mut parties: Vec<Node<'a>> = Vec::new();
    let mut cursor = controller.walk();
    for child in controller.children_by_field_name(fields::PARTY, &mut cursor) {
        if child.kind() != nodes::VARIABLE {
            // Mixed shapes (some variables, some parens) collapse the whole
            // site rather than emit a partial set.
            return Vec::new();
        }
        parties.push(child);
    }
    parties
}

/// Party-name swap candidates for a controller site. Combines the template's
/// `with`-block Party params with this choice's own Party params, drops
/// duplicates, then excludes names already named in this controller list.
fn swap_candidates(site: &ControllerSite, in_list: &[Node], source: &str) -> Vec<String> {
    let mut available_parties: Vec<String> = Vec::new();
    let mut push_unique = |name: &str| {
        if !available_parties.iter().any(|n| n == name) {
            available_parties.push(name.to_string());
        }
    };
    for n in collect_party_param_names(&site.template, source) {
        push_unique(&n);
    }
    for n in collect_party_param_names(&site.choice, source) {
        push_unique(&n);
    }
    let in_list_texts: Vec<&str> = in_list.iter().map(|n| node_text(n, source)).collect();
    available_parties
        .into_iter()
        .filter(|n| !in_list_texts.contains(&n.as_str()))
        .collect()
}

/// True when a `field_decl.type` node names the unqualified DAML
/// `Party` type or a module-qualified `M.Party` reference. The
/// grammar's `prefix_id` form is operator-only and cannot carry a
/// type constructor, so we only match `name` and `qualified`.
fn field_type_is_party(ty: Node, source: &str) -> bool {
    match ty.kind() {
        nodes::NAME => node_text(&ty, source) == "Party",
        nodes::QUALIFIED => ty
            .child_by_field_name(fields::ID)
            .is_some_and(|id| id.kind() == nodes::NAME && node_text(&id, source) == "Party"),
        _ => false,
    }
}

/// Party-typed parameter names directly declared inside `node`'s own
/// `with_fields` block (template or choice). Walks only the immediate
/// `with_fields` child; nested templates/choices have their own blocks and
/// are reached through different sites.
fn collect_party_param_names(node: &Node, source: &str) -> Vec<String> {
    let mut names = Vec::new();
    // A choice with no `with`-block has no choice-local params; this returns
    // empty cleanly without needing a separate codepath.
    let mut fields_cursor = node.walk();
    let Some(with_fields) = node
        .children_by_field_name(fields::FIELDS, &mut fields_cursor)
        .next()
    else {
        return names;
    };
    let mut cursor = with_fields.walk();
    for field in with_fields.children_by_field_name(fields::FIELD, &mut cursor) {
        if field.kind() != nodes::FIELD_DECL {
            continue;
        }
        let Some(ty) = field.child_by_field_name(fields::TYPE) else {
            continue;
        };
        if !field_type_is_party(ty, source) {
            continue;
        }
        let mut name_cursor = field.walk();
        for name in field.children_by_field_name(fields::NAME, &mut name_cursor) {
            if name.kind() == nodes::VARIABLE {
                names.push(node_text(&name, source).to_string());
            }
        }
    }
    names
}

/// Byte range to delete to remove `parties[idx]` from a multi-party
/// controller list. The range includes the adjacent comma+whitespace so the
/// resulting `controller` clause is well-formed.
///
/// First party owns the separator on its right (`a, b` -> `b`); every other
/// party owns the separator on its left (`a, b` -> `a`).
fn removal_byte_range(parties: &[Node], idx: usize) -> (usize, usize) {
    if idx == 0 {
        (parties[0].start_byte(), parties[1].start_byte())
    } else {
        (parties[idx - 1].end_byte(), parties[idx].end_byte())
    }
}

fn ancestor_of_kind<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cur = node.parent();
    while let Some(p) = cur {
        if p.kind() == kind {
            return Some(p);
        }
        cur = p.parent();
    }
    None
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

    // Every common mutation is either emitted or explicitly listed in
    // UNSUPPORTED_COMMON. New COMMON_MUTATIONs added upstream must be
    // implemented in DAML or annotated with a reason here.
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
