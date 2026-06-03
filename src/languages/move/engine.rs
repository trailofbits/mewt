use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::config::MoveDialect;
use crate::types::{Mutant, Mutation, Target};
use crate::utils::{node_text, parse_source};

use super::dialect::{MoveDialectConfig, config_for_dialect};
use super::mutations::MOVE_MUTATIONS;
use super::syntax::{MoveSyntax, syntax_for_dialect};

pub struct MoveDialectEngine {
    dialect: MoveDialect,
    canonical_name: &'static str,
    display_name: &'static str,
    config: MoveDialectConfig,
    syntax: MoveSyntax,
    mutations: Vec<Mutation>,
}

impl MoveDialectEngine {
    pub fn new(dialect: MoveDialect) -> Self {
        let config = config_for_dialect(dialect);
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend(
            COMMON_MUTATIONS
                .iter()
                .chain(MOVE_MUTATIONS.iter())
                .filter(|mutation| config.supports_mutation_slug(mutation.slug))
                .map(|mutation| Mutation {
                    slug: mutation.slug,
                    description: mutation.description,
                    severity: mutation.severity.clone(),
                }),
        );

        let (canonical_name, display_name) = match dialect {
            MoveDialect::Sui => ("Move/sui", "Sui Move"),
            MoveDialect::Iota => ("Move/iota", "IOTA Move"),
            MoveDialect::Aptos => ("Move/aptos", "Aptos Move"),
        };

        Self {
            dialect,
            canonical_name,
            display_name,
            config,
            syntax: syntax_for_dialect(dialect),
            mutations,
        }
    }

    pub fn dialect(&self) -> MoveDialect {
        self.dialect
    }
}

impl LanguageEngine for MoveDialectEngine {
    fn name(&self) -> &'static str {
        self.display_name
    }

    fn canonical_name(&self) -> &'static str {
        self.canonical_name
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        mutate_move_with_config(target, &self.config, self.syntax, &self.mutations)
    }
}

pub struct MoveLanguageEngine {
    inner: MoveDialectEngine,
}

impl Default for MoveLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveLanguageEngine {
    pub fn new() -> Self {
        Self {
            inner: MoveDialectEngine::new(MoveDialect::Sui),
        }
    }
}

impl LanguageEngine for MoveLanguageEngine {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn canonical_name(&self) -> &'static str {
        self.inner.canonical_name()
    }

    fn get_mutations(&self) -> &[Mutation] {
        self.inner.get_mutations()
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        self.inner.mutate(target)
    }
}

fn mutate_move_with_config(
    target: &Target,
    dialect_config: &MoveDialectConfig,
    syntax: MoveSyntax,
    mutations: &[Mutation],
) -> Vec<Mutant> {
    let source = &target.text;

    let tree = match parse_source(source, dialect_config.parser_language()) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let root = tree.root_node();
    let statement_kinds = syntax
        .block_item
        .map(|kind| vec![kind])
        .unwrap_or_else(|| vec![syntax.break_expression]);

    let mut all_mutants = Vec::new();
    for m in mutations {
        match m.slug {
            "ER" => {
                all_mutants.extend(
                    patterns::replace(
                        root,
                        source,
                        &statement_kinds,
                        dialect_config.abort_statement,
                        &|node, src| !node_text(node, src).contains("abort "),
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "ER")),
                );
            }
            "CR" => {
                all_mutants.extend(
                    patterns::wrap(root, source, &statement_kinds, "/* ", " */")
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "CR")),
                );
            }
            "IF" => all_mutants.extend(
                patterns::replace_condition(
                    root,
                    source,
                    syntax.if_expression,
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
                    syntax.if_expression,
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
                    syntax.while_expression,
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
                    &[syntax.break_expression, syntax.continue_expression],
                    &["break", "continue"],
                )
                .into_iter()
                .map(|p| Mutant::from_partial(p, target, "LC")),
            ),
            "BL" => all_mutants.extend(
                patterns::shuffle_nodes(root, source, &[syntax.bool_literal], &["true", "false"])
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BL")),
            ),
            "AOS" => all_mutants.extend(
                patterns::shuffle_operators(
                    root,
                    source,
                    &[syntax.binary_expression],
                    &["+", "-", "*", "/", "%"],
                )
                .into_iter()
                .map(|p| Mutant::from_partial(p, target, "AOS")),
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
                    &["==", "!=", "<", "<=", ">", ">="],
                )
                .into_iter()
                .map(|p| Mutant::from_partial(p, target, "COS")),
            ),
            "SOS" => all_mutants.extend(
                patterns::shuffle_operators(
                    root,
                    source,
                    &[syntax.binary_expression],
                    &["<<", ">>"],
                )
                .into_iter()
                .map(|p| Mutant::from_partial(p, target, "SOS")),
            ),
            "NR" => {
                if let (Some(operator_field), Some(operand_field)) =
                    (syntax.unary_operator_field, syntax.unary_operand_field)
                {
                    all_mutants.extend(
                        patterns::remove_unary_operator(
                            root,
                            source,
                            syntax.unary_not_expression,
                            operator_field,
                            operand_field,
                            "!",
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "NR")),
                    );
                }
            }
            _ => {
                panic!(
                    "Unknown mutation slug encountered in Move engine: {}",
                    m.slug
                );
            }
        }
    }
    all_mutants
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = MoveDialectEngine::new(MoveDialect::Sui);
        let mut seen: HashSet<&str> = HashSet::new();
        let mut dups: BTreeSet<String> = BTreeSet::new();
        for m in engine.get_mutations() {
            if !seen.insert(m.slug) {
                dups.insert(m.slug.to_string());
            }
        }
        assert!(
            dups.is_empty(),
            "Duplicate mutation slugs found in Move engine: {dups:?}",
        );
    }

    #[test]
    fn all_defined_slugs_have_match_arms() {
        let text = "module test::m { fun foo(): bool { true } }";
        let target = Target {
            id: 0,
            path: PathBuf::from("test.move"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "Move/sui".to_string(),
        };
        let engine = MoveDialectEngine::new(MoveDialect::Sui);
        let _ = engine.mutate(&target);
    }

    #[test]
    fn dialect_engine_catalog_excludes_unsupported_mutations() {
        let engine = MoveDialectEngine::new(MoveDialect::Sui);
        let slugs: Vec<_> = engine.get_mutations().iter().map(|m| m.slug).collect();
        assert!(!slugs.contains(&"AAOS"));
        assert!(!slugs.contains(&"BAOS"));
        assert!(!slugs.contains(&"SAOS"));
    }
}
