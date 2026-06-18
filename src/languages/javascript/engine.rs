use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Language, Mutant, Mutation, Target};
use crate::utils::{node_text, parse_source};

use super::dialect::{
    JavaScriptDialect, JavaScriptDialectConfig, config_for_dialect, language_name_for_dialect,
};
use super::mutations::JAVASCRIPT_MUTATIONS;
use super::syntax::{JavaScriptSyntax, syntax_for_dialect};

pub struct JavaScriptDialectEngine {
    language: Language,
    config: JavaScriptDialectConfig,
    syntax: JavaScriptSyntax,
    mutations: Vec<Mutation>,
}

impl JavaScriptDialectEngine {
    pub fn new(dialect: JavaScriptDialect) -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(JAVASCRIPT_MUTATIONS);

        Self {
            language: language_name_for_dialect(dialect)
                .parse()
                .expect("hardcoded language identifier should be valid"),
            config: config_for_dialect(dialect),
            syntax: syntax_for_dialect(dialect),
            mutations,
        }
    }

    pub fn dialect(&self) -> JavaScriptDialect {
        self.config.dialect
    }
}

impl LanguageEngine for JavaScriptDialectEngine {
    fn language(&self) -> &Language {
        &self.language
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;

        let tree = match parse_source(source, self.config.parser_language()) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let root = tree.root_node();
        let syntax = self.syntax;

        let mut all_mutants = Vec::new();
        for m in &self.mutations {
            match m.slug {
                "ER" => {
                    all_mutants.extend(
                        patterns::replace(
                            root,
                            source,
                            &[
                                syntax.expression_statement,
                                syntax.return_statement,
                                syntax.variable_declaration,
                                syntax.if_statement,
                                syntax.while_statement,
                                syntax.for_statement,
                                syntax.for_in_statement,
                                syntax.do_statement,
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
                                syntax.expression_statement,
                                syntax.return_statement,
                                syntax.variable_declaration,
                                syntax.if_statement,
                                syntax.while_statement,
                                syntax.for_statement,
                                syntax.for_in_statement,
                                syntax.do_statement,
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
                        syntax.if_statement,
                        syntax.condition_field,
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
                        syntax.if_statement,
                        syntax.condition_field,
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
                        syntax.while_statement,
                        syntax.condition_field,
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
                        &[syntax.call_expression],
                        syntax.arguments_field,
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AS")),
                ),
                "LC" => all_mutants.extend(
                    patterns::shuffle_nodes(
                        root,
                        source,
                        &[syntax.break_statement, syntax.continue_statement],
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
                        &[syntax.binary_expression],
                        &["+", "-", "*", "/", "%", "**"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AOS")),
                ),
                "AAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.augmented_assignment_expression],
                        &["+=", "-=", "*=", "/=", "%=", "**="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "AAOS")),
                ),
                "BOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.binary_expression],
                        &["&", "|", "^"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BOS")),
                ),
                "BAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.augmented_assignment_expression],
                        &["&=", "|=", "^="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BAOS")),
                ),
                "LOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.binary_expression],
                        &["&&", "||"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "LOS")),
                ),
                "COS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.binary_expression],
                        &["==", "!=", "===", "!==", "<", "<=", ">", ">="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "COS")),
                ),
                "SOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.binary_expression],
                        &["<<", ">>", ">>>"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SOS")),
                ),
                "SAOS" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.augmented_assignment_expression],
                        &["<<=", ">>=", ">>>="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "SAOS")),
                ),
                "NR" => all_mutants.extend(
                    patterns::remove_unary_operator(
                        root,
                        source,
                        syntax.unary_expression,
                        syntax.operator_field,
                        syntax.argument_field,
                        "!",
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "NR")),
                ),
                "NCR" => all_mutants.extend(
                    patterns::shuffle_operators(
                        root,
                        source,
                        &[syntax.binary_expression],
                        &["??", "||"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "NCR")),
                ),
                "AWR" => all_mutants.extend(
                    patterns::replace_with_first_named_child(root, source, syntax.await_expression)
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "AWR")),
                ),
                _ => panic!("Unknown mutation slug: {}", m.slug),
            }
        }

        all_mutants
    }
}

pub struct JavaScriptLanguageEngine {
    inner: JavaScriptDialectEngine,
}

impl Default for JavaScriptLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptLanguageEngine {
    pub fn new() -> Self {
        Self {
            inner: JavaScriptDialectEngine::new(JavaScriptDialect::JavaScript),
        }
    }
}

impl LanguageEngine for JavaScriptLanguageEngine {
    fn language(&self) -> &Language {
        self.inner.language()
    }

    fn get_mutations(&self) -> &[Mutation] {
        self.inner.get_mutations()
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        self.inner.mutate(target)
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
            language: "javascript".parse().unwrap(),
        };
        let engine = JavaScriptLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
