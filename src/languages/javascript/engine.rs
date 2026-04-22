use std::sync::OnceLock;
use tree_sitter::Language as TsLanguage;

use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, Target};
use crate::utils::{node_text, parse_source};

use super::mutations::JAVASCRIPT_MUTATIONS;
use super::syntax::{fields, nodes};

static JS_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static TS_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static TSX_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_javascript() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_typescript() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_tsx() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct JavaScriptLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for JavaScriptLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(JAVASCRIPT_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for JavaScriptLanguageEngine {
    fn name(&self) -> &'static str {
        "JavaScript"
    }

    fn extensions(&self) -> &[&'static str] {
        &["js", "ts", "jsx", "tsx"]
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        // Determine which grammar to use based on file extension
        let extension = target.path.extension().and_then(|e| e.to_str());

        let language = match extension {
            Some("ts") => TS_LANGUAGE
                .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_typescript()) }),
            Some("tsx") => {
                TSX_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_tsx()) })
            }
            // Default to JavaScript for .js, .jsx, and any other files
            _ => JS_LANGUAGE
                .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_javascript()) }),
        };

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
                                nodes::VARIABLE_DECLARATION,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::FOR_IN_STATEMENT,
                                nodes::DO_STATEMENT,
                            ],
                            "throw new Error(\"mewt\");",
                            &|node, src| {
                                let text = node_text(node, src);
                                // Do not replace statements that already contain an error
                                !text.contains("throw ")
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
                                nodes::VARIABLE_DECLARATION,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::FOR_IN_STATEMENT,
                                nodes::DO_STATEMENT,
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
                    patterns::swap_args(root, source, &[nodes::CALL_EXPRESSION], fields::ARGUMENTS)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AS")),
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
                "BL" => all_mutants.extend(
                    patterns::shuffle_nodes(root, source, &["true", "false"], &["true", "false"])
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "BL")),
                ),
                "AOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["+", "-", "*", "/", "%", "**"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AOS")),
                ),
                "AAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::AUGMENTED_ASSIGNMENT_EXPRESSION],
                        &["+=", "-=", "*=", "/=", "%=", "**="],
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
                "COS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["==", "!=", "===", "!==", "<", "<=", ">", ">="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "COS")),
                ),
                "SOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_EXPRESSION],
                        &["<<", ">>", ">>>"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SOS")),
                ),
                "SAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::AUGMENTED_ASSIGNMENT_EXPRESSION],
                        &["<<=", ">>=", ">>>="],
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
                        fields::ARGUMENT,
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
                            nodes::VARIABLE_DECLARATION,
                            nodes::IF_STATEMENT,
                            nodes::WHILE_STATEMENT,
                            nodes::FOR_STATEMENT,
                            nodes::FOR_IN_STATEMENT,
                            nodes::DO_STATEMENT,
                        ],
                        &javascript_enclosing_function,
                        &|func, src| javascript_early_return_replacement(func, src, extension),
                        &javascript_should_replace_for_ger,
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "GER")),
                ),
                _ => panic!("Unknown mutation slug: {}", m.slug),
            }
        }

        all_mutants
    }
}

fn javascript_enclosing_function<'a>(
    node: &tree_sitter::Node<'a>,
) -> Option<tree_sitter::Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            nodes::FUNCTION_DECLARATION
            | nodes::GENERATOR_FUNCTION_DECLARATION
            | nodes::METHOD_DEFINITION
            | nodes::FUNCTION
            | nodes::ARROW_FUNCTION => return Some(parent),
            _ => current = parent.parent(),
        }
    }
    None
}

fn javascript_early_return_replacement(
    func_node: &tree_sitter::Node,
    source: &str,
    extension: Option<&str>,
) -> Option<String> {
    match extension {
        Some("ts") | Some("tsx") => {
            let return_type_node = func_node.child_by_field_name(fields::RETURN_TYPE)?;
            let type_text = node_text(&return_type_node, source)
                .trim()
                .trim_start_matches(':')
                .trim()
                .to_ascii_lowercase();
            match type_text.as_str() {
                "void" => Some("return;".to_string()),
                "boolean" => Some("return false;".to_string()),
                "number" => Some("return 0;".to_string()),
                "string" => Some("return \"\";".to_string()),
                _ => None,
            }
        }
        _ => Some("return;".to_string()),
    }
}

fn javascript_should_replace_for_ger(node: &tree_sitter::Node, source: &str) -> bool {
    if node.kind() != nodes::EXPRESSION_STATEMENT {
        return true;
    }

    let text = node_text(node, source).trim_start();
    !text.starts_with("return ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = JavaScriptLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(dups.is_empty(), "Duplicate mutation slugs found: {dups:?}",);
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text = "function test() { if (true) return 42; }";
        let target = Target {
            id: 0,
            path: PathBuf::from("test.js"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "JavaScript".to_string(),
        };
        let engine = JavaScriptLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
