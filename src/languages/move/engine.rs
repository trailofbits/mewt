use crate::LanguageEngine;
use crate::mutations::COMMON_MUTATIONS;
use crate::patterns;
use crate::types::{Mutant, Mutation, Target};
use crate::utils::{node_text, parse_source};

use super::dialect::profile_for_target_language;
use super::mutations::MOVE_MUTATIONS;
use super::syntax::{fields, nodes};

pub struct MoveLanguageEngine {
    mutations: Vec<Mutation>,
}

impl Default for MoveLanguageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveLanguageEngine {
    pub fn new() -> Self {
        let mut mutations: Vec<Mutation> = Vec::new();
        mutations.extend_from_slice(COMMON_MUTATIONS);
        mutations.extend_from_slice(MOVE_MUTATIONS);
        Self { mutations }
    }
}

impl LanguageEngine for MoveLanguageEngine {
    fn name(&self) -> &'static str {
        "Move"
    }

    fn extensions(&self) -> &[&'static str] {
        &["move"]
    }

    fn get_mutations(&self) -> &[Mutation] {
        &self.mutations
    }

    fn mutate(&self, target: &Target) -> Vec<Mutant> {
        let source = &target.text;
        let profile = profile_for_target_language(&target.language);

        let tree = match parse_source(source, profile.parser_language()) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let root = tree.root_node();

        let mut all_mutants = Vec::new();
        for m in &self.mutations {
            if !profile.supports_mutation_slug(m.slug) {
                continue;
            }

            match m.slug {
                "ER" => {
                    all_mutants.extend(
                        patterns::replace(
                            root,
                            source,
                            // block_item includes the trailing semicolon
                            &[nodes::BLOCK_ITEM],
                            profile.abort_statement,
                            &|node, src| !node_text(node, src).contains("abort "),
                        )
                        .into_iter()
                        .map(|p| Mutant::from_partial(p, target, "ER")),
                    );
                }
                "CR" => {
                    all_mutants.extend(
                        patterns::wrap(root, source, &[nodes::BLOCK_ITEM], "/* ", " */")
                            .into_iter()
                            .map(|p| Mutant::from_partial(p, target, "CR")),
                    );
                }
                "IF" => all_mutants.extend(
                    patterns::replace_condition(
                        root,
                        source,
                        nodes::IF_EXPRESSION,
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
                        nodes::IF_EXPRESSION,
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
                        nodes::WHILE_EXPRESSION,
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
                        &[nodes::BREAK_EXPRESSION, nodes::CONTINUE_EXPRESSION],
                        &["break", "continue"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "LC")),
                ),
                "BL" => all_mutants.extend(
                    patterns::shuffle_nodes(
                        root,
                        source,
                        &[nodes::BOOL_LITERAL],
                        &["true", "false"],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "BL")),
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
                        &["==", "!=", "<", "<=", ">", ">="],
                    )
                    .into_iter()
                    .map(|p| Mutant::from_partial(p, target, "COS")),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    use std::path::PathBuf;

    #[test]
    fn no_duplicate_slugs_in_combined_mutations() {
        let engine = MoveLanguageEngine::new();
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
        let engine = MoveLanguageEngine::new();
        let _ = engine.mutate(&target);
    }

    #[test]
    fn iota_profile_is_accepted() {
        let text = "module test::m { fun foo(): bool { true } }";
        let target = Target {
            id: 0,
            path: PathBuf::from("test.move"),
            file_hash: crate::types::Hash::digest(text.to_string()),
            text: text.to_string(),
            language: "Move/iota".to_string(),
        };
        let engine = MoveLanguageEngine::new();
        let _ = engine.mutate(&target);
    }
}
