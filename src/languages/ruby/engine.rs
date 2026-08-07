use std::sync::OnceLock;
use tree_sitter::Language as TsLanguage;

use crate::LanguageEngine;
use crate::core::engine::patterns;
use crate::mutations::COMMON_MUTATIONS;
use crate::types::{Language, Mutant, Mutation, PartialMutant, Target};
use crate::utils::{node_text, parse_source};

use super::mutations::RUBY_MUTATIONS;
use super::syntax::{fields, nodes};

static RUBY_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

/// Statement-level nodes handled by both ER and CR. Container bodies
/// (method/do_block/block) are intentionally excluded so that the
/// outermost-match logic in `patterns` keeps producing one mutant per
/// inner statement rather than a single mutant for the whole body.
const STATEMENT_KINDS: &[&str] = &[
    nodes::CALL,
    nodes::ASSIGNMENT,
    nodes::RETURN,
    nodes::BREAK,
    nodes::NEXT,
    nodes::REDO,
    nodes::RETRY,
    nodes::YIELD,
    nodes::SUPER,
    nodes::RESCUE_MODIFIER,
    nodes::IF,
    nodes::UNLESS,
    nodes::WHILE,
    nodes::UNTIL,
    nodes::FOR,
    nodes::CONDITIONAL,
    nodes::IF_MODIFIER,
    nodes::UNLESS_MODIFIER,
    nodes::WHILE_MODIFIER,
    nodes::UNTIL_MODIFIER,
    nodes::BEGIN,
    nodes::CASE,
    nodes::CASE_MATCH,
];

/// Parent node kinds that indicate expression position — a node whose parent
/// is one of these is a sub-expression, not a statement, and must not be
/// wrapped or replaced by CR/ER (which produce non-expression text).
const EXPRESSION_POSITION_PARENTS: &[&str] = &[
    nodes::BINARY,
    nodes::INTERPOLATION,
    nodes::PAIR,
    nodes::OPTIONAL_PARAMETER,
    nodes::KEYWORD_PARAMETER,
    nodes::ARRAY,
    nodes::UNARY,
    nodes::ELEMENT_REFERENCE,
    nodes::OPERATOR_ASSIGNMENT,
];

unsafe extern "C" {
    fn tree_sitter_ruby() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct RubyLanguageEngine {
    language: Language,
    mutations: Vec<Mutation>,
}

impl Default for RubyLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RubyLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(RUBY_MUTATIONS);
        Self {
            language: "ruby"
                .parse()
                .expect("hardcoded language identifier should be valid"),
            mutations,
        }
    }
}

impl LanguageEngine for RubyLanguageEngine {
    fn language(&self) -> &Language {
        &self.language
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        let language =
            RUBY_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_ruby()) });
        let tree = match parse_source(source, language) {
            Some(tree) => tree,
            None => return Vec::new(),
        };
        let root = tree.root_node();

        let mut all_mutants = Vec::new();
        for mutation in &self.mutations {
            match mutation.slug {
                "ER" => {
                    all_mutants.extend(
                        patterns::replace(
                            root,
                            source,
                            STATEMENT_KINDS,
                            "raise \"mewt\"",
                            &|node, src| {
                                let text = node_text(node, src);
                                if text.contains("raise") {
                                    return false;
                                }
                                // Skip nodes that are sub-expressions; raise "mewt" is a
                                // valid expression but replacing an operand inside a larger
                                // expression changes program flow in unexpected ways.
                                if let Some(parent) = node.parent() {
                                    if EXPRESSION_POSITION_PARENTS.contains(&parent.kind()) {
                                        return false;
                                    }
                                }
                                true
                            },
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "ER")),
                    );
                }
                "CR" => {
                    all_mutants.extend(
                        patterns::replace(root, source, STATEMENT_KINDS, "nil", &|node, _src| {
                            // Skip sub-expressions to avoid redundant mutants; `nil` is a
                            // valid expression but replacing an operand inside a larger
                            // expression is already covered by the enclosing statement.
                            if let Some(parent) = node.parent() {
                                if EXPRESSION_POSITION_PARENTS.contains(&parent.kind()) {
                                    return false;
                                }
                            }
                            true
                        })
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "CR")),
                    );
                }
                "IF" => {
                    for node_kind in [
                        nodes::IF,
                        nodes::IF_MODIFIER,
                        nodes::ELSIF,
                        nodes::IF_GUARD,
                        nodes::CONDITIONAL,
                    ] {
                        all_mutants.extend(
                            patterns::replace_condition(
                                root,
                                source,
                                node_kind,
                                fields::CONDITION,
                                &["if", "elsif"],
                                "false",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "IF")),
                        );
                    }
                }
                "IT" => {
                    for node_kind in [
                        nodes::IF,
                        nodes::IF_MODIFIER,
                        nodes::ELSIF,
                        nodes::IF_GUARD,
                        nodes::CONDITIONAL,
                    ] {
                        all_mutants.extend(
                            patterns::replace_condition(
                                root,
                                source,
                                node_kind,
                                fields::CONDITION,
                                &["if", "elsif"],
                                "true",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "IT")),
                        );
                    }
                }
                "WF" => {
                    for node_kind in [nodes::WHILE, nodes::WHILE_MODIFIER] {
                        all_mutants.extend(
                            patterns::replace_condition(
                                root,
                                source,
                                node_kind,
                                fields::CONDITION,
                                &["while"],
                                "false",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "WF")),
                        );
                    }
                }
                "AS" => {
                    all_mutants.extend(
                        patterns::swap_args(root, source, &[nodes::CALL], fields::ARGUMENTS)
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "AS")),
                    );
                    all_mutants.extend(
                        patterns::swap_named_children(root, source, &[nodes::ARRAY_PATTERN])
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "AS")),
                    );
                }
                "LC" => {
                    all_mutants.extend(
                        patterns::shuffle_nodes(
                            root,
                            source,
                            &[nodes::BREAK, nodes::NEXT, nodes::REDO, nodes::RETRY],
                            &["break", "next", "redo", "retry"],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "LC")),
                    );
                }
                "BL" => {
                    all_mutants.extend(
                        patterns::shuffle_nodes(
                            root,
                            source,
                            &[nodes::TRUE, nodes::FALSE],
                            &["true", "false"],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "BL")),
                    );
                }
                "AOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::BINARY],
                            &["+", "-", "*", "/", "%", "**"],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AOS")),
                    );
                }
                "AAOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::OPERATOR_ASSIGNMENT],
                            &["+=", "-=", "*=", "/=", "%=", "**="],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AAOS")),
                    );
                }
                "BOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::BINARY],
                            &["&", "|", "^"],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "BOS")),
                    );
                }
                "CES" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(root, source, &[nodes::BINARY], &["===", "=="])
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "CES")),
                    );
                }
                "BAOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::OPERATOR_ASSIGNMENT],
                            &["&=", "|=", "^="],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "BAOS")),
                    );
                }
                "EL" => {
                    let mut cursor = root.walk();
                    crate::utils::visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
                        if !crate::utils::is_in_comment(&node) {
                            let text = crate::utils::node_text(&node, source);
                            let replacement = match node.kind() {
                                nodes::STRING if text != "\"\"" && text != "''" => Some("\"\""),
                                nodes::ARRAY if text != "[]" => Some("[]"),
                                nodes::HASH if text != "{}" => Some("{}"),
                                _ => None,
                            };
                            if let Some(r) = replacement {
                                all_mutants.push(Mutant::from_partial(
                                    PartialMutant {
                                        byte_offset: node.start_byte() as u32,
                                        line_offset: crate::utils::calculate_line_offset(
                                            source,
                                            node.start_byte(),
                                        ),
                                        old_text: text.to_string(),
                                        new_text: r.to_string(),
                                    },
                                    target,
                                    "EL",
                                ));
                            }
                        }
                    });
                }
                "LAOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::OPERATOR_ASSIGNMENT],
                            &["||=", "&&="],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "LAOS")),
                    );
                }
                "LOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::BINARY],
                            &["&&", "||", "and", "or"],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "LOS")),
                    );
                }
                "COS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::BINARY],
                            &["==", "!=", "<", "<=", ">", ">="],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "COS")),
                    );
                }
                "SOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(root, source, &[nodes::BINARY], &["<<", ">>"])
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "SOS")),
                    );
                }
                "SAOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(
                            root,
                            source,
                            &[nodes::OPERATOR_ASSIGNMENT],
                            &["<<=", ">>="],
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "SAOS")),
                    );
                }
                "RMOS" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(root, source, &[nodes::BINARY], &["=~", "!~"])
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "RMOS")),
                    );
                }
                "NR" => {
                    for op in ["!", "not"] {
                        all_mutants.extend(
                            patterns::remove_unary_operator(
                                root,
                                source,
                                nodes::UNARY,
                                fields::OPERATOR,
                                fields::OPERAND,
                                op,
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "NR")),
                        );
                    }
                    all_mutants.extend(
                        patterns::remove_unary_operator(
                            root,
                            source,
                            nodes::EXPRESSION_REFERENCE_PATTERN,
                            "", // The ^ is not a named field, we can use a custom text replacement
                            fields::VALUE,
                            "^",
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "NR")),
                    );
                }
                "RBR" => {
                    all_mutants.extend(
                        patterns::shuffle_operators(root, source, &[nodes::RANGE], &["..", "..."])
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "RBR")),
                    );
                }
                "SNR" => {
                    let mut cursor = root.walk();
                    crate::utils::visit_nodes_with_cursor(root, &mut cursor, &mut |node| {
                        if node.kind() == nodes::CALL && !crate::utils::is_in_comment(&node) {
                            let mut nc = node.walk();
                            for child in node.children(&mut nc) {
                                if !child.is_named()
                                    && crate::utils::node_text(&child, source) == "&."
                                {
                                    all_mutants.push(Mutant::from_partial(
                                        PartialMutant {
                                            byte_offset: child.start_byte() as u32,
                                            line_offset: crate::utils::calculate_line_offset(
                                                source,
                                                child.start_byte(),
                                            ),
                                            old_text: "&.".to_string(),
                                            new_text: ".".to_string(),
                                        },
                                        target,
                                        "SNR",
                                    ));
                                    break; // Only mutate the operator
                                }
                            }
                        }
                    });
                }
                "UF" => {
                    for node_kind in [nodes::UNLESS, nodes::UNLESS_MODIFIER, nodes::UNLESS_GUARD] {
                        all_mutants.extend(
                            patterns::replace_condition(
                                root,
                                source,
                                node_kind,
                                fields::CONDITION,
                                &["unless"],
                                "false",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "UF")),
                        );
                    }
                }
                "UT" => {
                    for node_kind in [nodes::UNLESS, nodes::UNLESS_MODIFIER, nodes::UNLESS_GUARD] {
                        all_mutants.extend(
                            patterns::replace_condition(
                                root,
                                source,
                                node_kind,
                                fields::CONDITION,
                                &["unless"],
                                "true",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "UT")),
                        );
                    }
                }
                "ULF" => {
                    for node_kind in [nodes::UNTIL, nodes::UNTIL_MODIFIER] {
                        all_mutants.extend(
                            patterns::replace_condition(
                                root,
                                source,
                                node_kind,
                                fields::CONDITION,
                                &["until"],
                                "false",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "ULF")),
                        );
                    }
                }
                "ULT" => {
                    for node_kind in [nodes::UNTIL, nodes::UNTIL_MODIFIER] {
                        all_mutants.extend(
                            patterns::replace_condition(
                                root,
                                source,
                                node_kind,
                                fields::CONDITION,
                                &["until"],
                                "true",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "ULT")),
                        );
                    }
                }
                "UP" => {
                    for (node_kind, operand_field) in [
                        (nodes::VARIABLE_REFERENCE_PATTERN, "name"),
                        (nodes::EXPRESSION_REFERENCE_PATTERN, ""),
                    ] {
                        all_mutants.extend(
                            patterns::remove_unary_operator(
                                root,
                                source,
                                node_kind,
                                "",
                                operand_field,
                                "^",
                            )
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "UP")),
                        );
                    }
                }
                _ => {
                    panic!(
                        "Unknown mutation slug encountered in Ruby engine: {}",
                        mutation.slug
                    );
                }
            }
        }

        all_mutants
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = RubyLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in Ruby engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text = r#"
def process(value)
  if value > 0
    value + 1
  else
    value - 1
  end
end
"#;
        let target = Target {
            id: 0,
            path: PathBuf::from("test.rb"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "ruby".parse().unwrap(),
        };
        let engine = RubyLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
