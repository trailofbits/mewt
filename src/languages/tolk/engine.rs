use std::sync::OnceLock;
use tree_sitter::Language as TsLanguage;

use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, Target};
use crate::utils::{node_text, parse_source};

use super::mutations::TOLK_MUTATIONS;
use super::syntax::{fields, nodes};

static TOLK_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_tolk() -> *const tree_sitter::ffi::TSLanguage;
}

pub struct TolkLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for TolkLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TolkLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(TOLK_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for TolkLanguageEngine {
    fn name(&self) -> &'static str {
        "Tolk"
    }

    fn extensions(&self) -> &[&'static str] {
        &["tolk"]
    }

    fn tree_sitter_language(&self) -> TsLanguage {
        TOLK_LANGUAGE
            .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_tolk()) })
            .clone()
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn apply_all_mutations(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;
        let tree = match parse_source(source, &self.tree_sitter_language()) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let root = tree.root_node();

        let statement_kinds: &[&str] = &[
            nodes::EXPRESSION_STATEMENT,
            nodes::RETURN_STATEMENT,
            nodes::THROW_STATEMENT,
            nodes::LOCAL_VARS_DECLARATION,
            nodes::IF_STATEMENT,
            nodes::WHILE_STATEMENT,
            nodes::DO_WHILE_STATEMENT,
            nodes::REPEAT_STATEMENT,
        ];

        let mut all_mutants = Vec::new();
        for m in &self.mutations {
            match m.slug {
                "ER" => {
                    all_mutants.extend(
                        patterns::replace(
                            root,
                            source,
                            statement_kinds,
                            "throw 0",
                            &|node, src| {
                                let text = node_text(node, src);
                                !text.contains("throw")
                            },
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "ER")),
                    );
                }
                "CR" => {
                    all_mutants.extend(
                        patterns::wrap(root, source, statement_kinds, "/* ", " */")
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
                    patterns::swap_args(root, source, &[nodes::FUNCTION_CALL], fields::ARGUMENTS)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AS")),
                ),
                "AOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_OPERATOR],
                        &["+", "-", "*", "/", "%"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AOS")),
                ),
                "AAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::SET_ASSIGNMENT],
                        &["+=", "-=", "*=", "/="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AAOS")),
                ),
                "BOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_OPERATOR],
                        &["&", "|", "^"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BOS")),
                ),
                "BAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::SET_ASSIGNMENT],
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
                        &[nodes::BINARY_OPERATOR],
                        &["==", "!=", "<", "<=", ">", ">="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "COS")),
                ),
                "LOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_OPERATOR],
                        &["&&", "||"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "LOS")),
                ),
                "SOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::BINARY_OPERATOR],
                        &["<<", ">>"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SOS")),
                ),
                "SAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[nodes::SET_ASSIGNMENT],
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
                _ => {
                    panic!(
                        "Unknown mutation slug encountered in Tolk engine: {}",
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
        let engine = TolkLanguageEngine::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in Tolk engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text: &str = r#"fun test(a: int, b: int): int {
    if (a > b) {
        return a - b;
    }
    return a + b;
}
"#;
        let target = Target {
            id: 0,
            path: PathBuf::from("test.tolk"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "Tolk".to_string(),
        };
        let engine = TolkLanguageEngine::new();
        let _ = engine.apply_all_mutations(&target);
    }
}
