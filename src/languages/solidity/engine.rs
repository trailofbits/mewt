use std::sync::OnceLock;
use tree_sitter::{Language as TsLanguage, Node};

use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, PartialMutant, Target};
use crate::utils::{
    calculate_line_offset, is_in_comment, node_text, parse_source, visit_nodes_with_cursor,
};

use super::mutations::SOLIDITY_MUTATIONS;
use super::syntax::{fields, nodes};

static SOLIDITY_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_solidity() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct SolidityLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for SolidityLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SolidityLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(SOLIDITY_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for SolidityLanguageEngine {
    fn name(&self) -> &'static str {
        "Solidity"
    }

    fn extensions(&self) -> &[&'static str] {
        &["sol"]
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        // Load grammar once and cache it
        let language = SOLIDITY_LANGUAGE
            .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_solidity()) });

        let tree = match parse_source(source, language) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let root = tree.root_node();

        let mut all_mutants = Vec::new();
        for m in &self.mutations {
            match m.slug {
                "ER" => {
                    all_mutants.extend(
                        patterns::replace(
                            root,
                            source,
                            &[
                                nodes::EXPRESSION_STATEMENT,
                                nodes::RETURN_STATEMENT,
                                nodes::LET_STATEMENT,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                            ],
                            "require(false);",
                            &|node, src| {
                                let text = node_text(node, src);
                                // Avoid replacing statements already containing a require
                                !text.contains("require(")
                            },
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "ER")),
                    );
                }
                "CR" => {
                    all_mutants.extend(
                        patterns::wrap(
                            root,
                            source,
                            &[
                                nodes::EXPRESSION_STATEMENT,
                                nodes::RETURN_STATEMENT,
                                nodes::LET_STATEMENT,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                            ],
                            "/* ",
                            " */",
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "CR")),
                    );
                }
                "IF" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::IF_STATEMENT,
                        fields::CONDITION,
                        &["if"],
                        "false",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "IF")),
                ),
                "IT" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::IF_STATEMENT,
                        fields::CONDITION,
                        &["if"],
                        "true",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "IT")),
                ),
                "WF" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::WHILE_STATEMENT,
                        fields::CONDITION,
                        &["while"],
                        "false",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "WF")),
                ),
                "AS" => all_mutants.extend(
                    patterns::swap_args(
                        root,
                        source,
                        &[nodes::METHOD_CALL_EXPRESSION, nodes::STATIC_CALL_EXPRESSION],
                        fields::ARGUMENTS,
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AS")),
                ),
                // Shared operator shuffles
                "AOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["+", "-", "*", "/"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AOS")),
                ),
                "AAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::AUGMENTED_ASSIGNMENT_EXPRESSION],
                        &["+=", "-=", "*=", "/=", "%="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AAOS")),
                ),
                "BOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["&", "|", "^"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BOS")),
                ),
                "BAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::AUGMENTED_ASSIGNMENT_EXPRESSION],
                        &["&=", "|=", "^="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BAOS")),
                ),
                "BL" => all_mutants.extend(
                    patterns::shuffle_nodes(root, source, &[nodes::BOOLEAN], &["true", "false"])
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "BL")),
                ),
                "COS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["==", "!=", "<", "<=", ">", ">="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "COS")),
                ),
                "LOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["&&", "||"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "LOS")),
                ),
                "SOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["<<", ">>"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SOS")),
                ),
                "SAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::AUGMENTED_ASSIGNMENT_EXPRESSION],
                        &["<<=", ">>="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SAOS")),
                ),
                "LC" => all_mutants.extend(
                    patterns::shuffle_nodes(
                        root,
                        source,
                        &[nodes::BREAK_STATEMENT, nodes::CONTINUE_STATEMENT],
                        &["break", "continue"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "LC")),
                ),
                "NR" => all_mutants.extend(
                    patterns::remove_unary_operator(
                        root,
                        source,
                        nodes::UNARY_EXPRESSION,
                        fields::OPERATOR,
                        fields::ARGUMENT,
                        "!",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "NR")),
                ),
                "RCI" => all_mutants.extend(
                    require_condition_inversion_mutants(root, source)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "RCI")),
                ),
                _ => {
                    panic!(
                        "Unknown mutation slug encountered in Solidity engine: {}",
                        m.slug
                    );
                }
            }
        }
        all_mutants
    }
}

/// Generate RCI (Require Condition Inversion) mutants for Solidity.
/// Finds `require(condition)` and `assert(condition)` calls and inverts the
/// condition: `condition` → `!(condition)`.
///
/// Skips conditions that are already negated (`!expr`) to avoid generating
/// duplicates with the NR (Negation Removal) mutator.
fn require_condition_inversion_mutants(root: Node, source: &str) -> Vec<PartialMutant> {
    let mut mutants = Vec::new();
    let mut cursor = root.walk();
    visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
        if node.kind() != "call_expression" || is_in_comment(&node) {
            return;
        }

        // Check if the callee is "require" or "assert"
        let mut nc = node.walk();
        let callee = node
            .named_children(&mut nc)
            .find(|c| c.kind() == "identifier" || c.kind() == "expression");
        let callee_text = match callee {
            Some(c) => node_text(&c, source),
            None => return,
        };
        if callee_text != "require" && callee_text != "assert" {
            return;
        }

        // Get the first call_argument — the condition
        let mut nc2 = node.walk();
        let first_arg = match node
            .named_children(&mut nc2)
            .find(|c| c.kind() == "call_argument")
        {
            Some(arg) => arg,
            None => return,
        };

        // Drill into the expression inside the call_argument
        let mut nc3 = first_arg.walk();
        let condition = match first_arg.named_children(&mut nc3).next() {
            Some(expr) => expr,
            None => return,
        };

        // Get the innermost meaningful expression (unwrap "expression" wrappers)
        let inner = unwrap_expression(&condition);

        // Skip if already negated — NR handles that case
        if inner.kind() == "unary_expression" {
            let mut uc = inner.walk();
            let first_child = inner.children(&mut uc).next();
            if let Some(op) = first_child {
                if node_text(&op, source) == "!" {
                    return;
                }
            }
        }

        // Skip simple comparisons — COS already shuffles the operator, and
        // inverting e.g. `x > 0` is equivalent to COS producing `x <= 0`
        if inner.kind() == "binary_expression" {
            let mut bc = inner.walk();
            let has_comparison_op = inner.children(&mut bc).any(|c| {
                let t = node_text(&c, source);
                matches!(t, "==" | "!=" | "<" | "<=" | ">" | ">=")
            });
            if has_comparison_op {
                return;
            }
        }

        let cond_text = node_text(&condition, source);
        mutants.push(PartialMutant {
            byte_offset: condition.start_byte() as u32,
            line_offset: calculate_line_offset(source, condition.start_byte()),
            old_text: cond_text.to_string(),
            new_text: format!("!({cond_text})"),
        });
    });
    mutants
}

/// Unwrap nested "expression" wrapper nodes to get the actual expression.
fn unwrap_expression<'a>(node: &Node<'a>) -> Node<'a> {
    let mut current = *node;
    while current.kind() == "expression" {
        let mut cursor = current.walk();
        match current.named_children(&mut cursor).next() {
            Some(child) => current = child,
            None => break,
        }
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = SolidityLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in Solidity engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text: &str = "contract C { function f(uint a, uint b) public { if (a > b) { return; } foo(1, 2); } }";
        let target = Target {
            id: 0,
            path: PathBuf::from("tests/examples/solidity/hello-world.sol"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "Solidity".to_string(),
        };
        let engine = SolidityLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
