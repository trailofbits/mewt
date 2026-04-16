use std::sync::OnceLock;
use tree_sitter::Language as TsLanguage;

use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, Target};
use crate::utils::{node_text, parse_source};

use super::mutations::CPP_MUTATIONS;
use super::syntax::{fields, nodes};

static CPP_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_cpp() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct CppLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for CppLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CppLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(CPP_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for CppLanguageEngine {
    fn name(&self) -> &'static str {
        "C++"
    }

    fn extensions(&self) -> &[&'static str] {
        &["cpp", "cc", "cxx", "hpp", "hxx"]
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        let language =
            CPP_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_cpp()) });

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
                                nodes::DECLARATION,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::FOR_RANGE_LOOP,
                                nodes::DO_STATEMENT,
                            ],
                            "throw std::runtime_error(\"mewt\");",
                            &|node, src| {
                                let text = node_text(node, src);
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
                                nodes::DECLARATION,
                                nodes::IF_STATEMENT,
                                nodes::WHILE_STATEMENT,
                                nodes::FOR_STATEMENT,
                                nodes::FOR_RANGE_LOOP,
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
                "WF" => {
                    all_mutants.extend(
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
                    );
                    all_mutants.extend(
                        patterns::replace_condition(
                            root,
                            source,
                            nodes::DO_STATEMENT,
                            fields::CONDITION,
                            &["while"],
                            "false",
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "WF")),
                    );
                }
                "AS" => all_mutants.extend(
                    patterns::swap_args(root, source, &[nodes::CALL_EXPRESSION], fields::ARGUMENTS)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AS")),
                ),
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
                "AAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::ASSIGNMENT_EXPRESSION],
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
                        &[nodes::ASSIGNMENT_EXPRESSION],
                        &["&=", "|=", "^="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BAOS")),
                ),
                "BL" => all_mutants.extend(
                    patterns::shuffle_nodes(
                        root,
                        source,
                        &[nodes::BOOLEAN, nodes::BOOLEAN_FALSE],
                        &["true", "false"],
                    )
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
                        &[nodes::ASSIGNMENT_EXPRESSION],
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
                        &["break;", "continue;"],
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
                // Not applicable to C++
                "RZ" | "RDV" | "RCI" => {}
                _ => {
                    panic!(
                        "Unknown mutation slug encountered in C++ engine: {}",
                        m.slug
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
        let engine = CppLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in C++ engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text = "int main() { if (true) return 42; }";
        let target = Target {
            id: 0,
            path: PathBuf::from("test.cpp"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "C++".to_string(),
        };
        let engine = CppLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
