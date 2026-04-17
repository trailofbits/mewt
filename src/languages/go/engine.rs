use std::sync::OnceLock;
use tree_sitter::Language as TsLanguage;

use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, Target};
use crate::utils::{node_text, parse_source};

use super::mutations::GO_MUTATIONS;
use super::syntax::{fields, nodes};

static GO_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_go() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct GoLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for GoLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GoLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(GO_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for GoLanguageEngine {
    fn name(&self) -> &'static str {
        "Go"
    }

    fn extensions(&self) -> &[&'static str] {
        &["go"]
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        // Load grammar once and cache it
        let language =
            GO_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_go()) });

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
                                nodes::IF_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::ASSIGNMENT_STATEMENT,
                                nodes::SHORT_VAR_DECLARATION,
                                nodes::INC_STATEMENT,
                                nodes::DEC_STATEMENT,
                            ],
                            "panic(\"mewt\")",
                            &|node, src| {
                                let text = node_text(node, src);
                                // Do not replace statements that already contain a panic
                                !text.contains("panic(")
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
                                nodes::IF_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::ASSIGNMENT_STATEMENT,
                                nodes::SHORT_VAR_DECLARATION,
                                nodes::INC_STATEMENT,
                                nodes::DEC_STATEMENT,
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
                "AS" => all_mutants.extend(
                    patterns::swap_args(root, source, &[nodes::CALL_EXPRESSION], fields::ARGUMENTS)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AS")),
                ),
                // Shared operator shuffles
                "AOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["+", "-", "*", "/", "%"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AOS")),
                ),
                "BOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["&", "|", "^", "&^"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BOS")),
                ),
                "BL" => all_mutants.extend(
                    patterns::shuffle_nodes(root, source, &["true", "false"], &["true", "false"])
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
                "AAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::ASSIGNMENT_STATEMENT],
                        &["+=", "-=", "*=", "/=", "%="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AAOS")),
                ),
                "BAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::ASSIGNMENT_STATEMENT],
                        &["&=", "|=", "^=", "&^="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BAOS")),
                ),
                "SAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::ASSIGNMENT_STATEMENT],
                        &["<<=", ">>="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SAOS")),
                ),
                "NR" => all_mutants.extend(
                    patterns::remove_unary_operator(
                        root,
                        source,
                        nodes::UNARY_EXPRESSION,
                        fields::OPERATOR,
                        fields::OPERAND,
                        "!",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "NR")),
                ),
                "GER" => all_mutants.extend(
                    patterns::replace_with_early_return(
                        root,
                        source,
                        &[
                            nodes::EXPRESSION_STATEMENT,
                            nodes::IF_STATEMENT,
                            nodes::FOR_STATEMENT,
                            nodes::ASSIGNMENT_STATEMENT,
                            nodes::SHORT_VAR_DECLARATION,
                            nodes::INC_STATEMENT,
                            nodes::DEC_STATEMENT,
                        ],
                        &go_enclosing_function,
                        &|func, src| go_early_return_replacement(func, src),
                        &|_, _| true,
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "GER")),
                ),
                // Mutations not applicable to Go
                "WF" | "RZ" => {
                    // Go has no `while` keyword (`WF`); `RZ` is a dead slug not in any mutation list.
                }
                _ => {
                    panic!("Unknown mutation slug encountered in Go engine: {}", m.slug);
                }
            }
        }
        all_mutants
    }
}

fn go_enclosing_function<'a>(node: &tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            nodes::FUNCTION_DECLARATION | nodes::METHOD_DECLARATION | nodes::FUNC_LITERAL => {
                return Some(parent);
            }
            _ => current = parent.parent(),
        }
    }
    None
}

fn go_early_return_replacement(func_node: &tree_sitter::Node, source: &str) -> Option<String> {
    let result_node = match func_node.child_by_field_name(fields::RESULT) {
        None => return Some("return".to_string()),
        Some(node) => node,
    };

    let defaults = if result_node.kind() == nodes::PARAMETER_LIST {
        let mut values = Vec::new();
        let mut cursor = result_node.walk();
        for child in result_node.named_children(&mut cursor) {
            match child.kind() {
                nodes::PARAMETER_DECLARATION | nodes::VARIADIC_PARAMETER_DECLARATION => {
                    let type_node = child.child_by_field_name(fields::TYPE)?;
                    values.push(go_type_default(&type_node, source)?);
                }
                _ => {}
            }
        }
        if values.is_empty() {
            return None;
        }
        values
    } else {
        vec![go_type_default(&result_node, source)?]
    };

    Some(format!("return {}", defaults.join(", ")))
}

fn go_type_default(type_node: &tree_sitter::Node, source: &str) -> Option<String> {
    let type_text = node_text(type_node, source).trim();
    let normalized = type_text.trim_start_matches("...").trim();

    match type_node.kind() {
        nodes::POINTER_TYPE
        | nodes::SLICE_TYPE
        | nodes::MAP_TYPE
        | nodes::CHANNEL_TYPE
        | nodes::INTERFACE_TYPE
        | nodes::FUNCTION_TYPE => Some("nil".to_string()),
        _ => match normalized {
            "bool" => Some("false".to_string()),
            "string" => Some("\"\"".to_string()),
            "float32" | "float64" => Some("0.0".to_string()),
            "int" | "int8" | "int16" | "int32" | "int64" | "uint" | "uint8" | "uint16"
            | "uint32" | "uint64" | "uintptr" | "byte" | "rune" => Some("0".to_string()),
            "error" => Some("nil".to_string()),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = GoLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in Go engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        // Use a simple Go program for smoke testing
        let text: &str = r#"package main

func main() {
    println("Hello")
}
"#;
        let target = Target {
            id: 0,
            path: PathBuf::from("test.go"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "Go".to_string(),
        };
        let engine = GoLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
