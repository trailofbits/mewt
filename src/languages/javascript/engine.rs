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

    fn javascript_language(&self) -> TsLanguage {
        JS_LANGUAGE
            .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_javascript()) })
            .clone()
    }

    fn typescript_language(&self) -> TsLanguage {
        TS_LANGUAGE
            .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_typescript()) })
            .clone()
    }

    fn tsx_language(&self) -> TsLanguage {
        TSX_LANGUAGE
            .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_tsx()) })
            .clone()
    }

    fn get_extension(target: &Target) -> Option<String> {
        target
            .path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string())
    }
}

impl LanguageEngine for JavaScriptLanguageEngine {
    fn name(&self) -> &'static str {
        "JavaScript"
    }

    fn extensions(&self) -> &[&'static str] {
        &["js", "ts", "jsx", "tsx"]
    }

    fn tree_sitter_language(&self) -> TsLanguage {
        // Default to JavaScript for compatibility
        self.javascript_language()
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn apply_all_mutations(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;
        let language = match Self::get_extension(target).as_deref() {
            Some("ts") => self.typescript_language(),
            Some("tsx") => self.tsx_language(),
            Some("jsx") => self.javascript_language(), // JSX uses JS grammar
            _ => self.javascript_language(),           // Default to JS
        };
        let tree = match parse_source(source, &language) {
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
                        &[nodes::BINARY_EXPRESSION],
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
                        &[nodes::BINARY_EXPRESSION],
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
                        &[nodes::BINARY_EXPRESSION],
                        &["<<=", ">>=", ">>>="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SAOS")),
                ),
                _ => panic!("Unknown mutation slug: {}", m.slug),
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
        let _ = engine.apply_all_mutations(&target);
    }
}
