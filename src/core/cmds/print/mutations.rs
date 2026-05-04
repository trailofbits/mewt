use std::collections::HashMap;

use log::{info, warn};
use serde::Serialize;

use crate::LanguageRegistry;
use crate::core::cmds::print::MutationsFilters;
use crate::types::config::config;
use crate::types::{Mutation, MutationSeverity};

#[derive(Serialize)]
struct JsonMutations {
    mutations: Vec<Mutation>,
}

pub async fn execute(filters: MutationsFilters, registry: &LanguageRegistry) -> Result<(), String> {
    let language = filters.language;
    let is_json_format = filters.format == "json";

    if language.as_deref().is_some_and(is_move_language_name)
        || (language.is_none() && filters.dialect.is_some())
    {
        let resolved_dialect = config()
            .resolve_move_dialect(filters.dialect.as_deref())
            .map_err(|e| e.to_string())?;
        if resolved_dialect.defaulted {
            warn!(
                "Move dialect not explicitly set; defaulting to '{}'. Use --dialect or [languages.move].dialect to select sui|iota|auto explicitly.",
                resolved_dialect.dialect.as_str()
            );
        } else {
            info!(
                "Using Move dialect '{}' for mutation listing",
                resolved_dialect.dialect.as_str()
            );
        }
    }
    if is_json_format {
        // Collect all mutations for JSON format
        let mut all_mutations = Vec::new();
        match &language {
            Some(lang_str) => {
                let mutation_engine = registry
                    .get_engine(lang_str)
                    .ok_or_else(|| format!("No engine found for language: {}", lang_str))?;
                all_mutations.extend(mutation_engine.get_mutations().iter().map(|m| Mutation {
                    slug: m.slug,
                    description: m.description,
                    severity: m.severity.clone(),
                }));
            }
            None => {
                for lang_name in registry.all_languages() {
                    let mutation_engine = registry
                        .get_engine(lang_name)
                        .ok_or_else(|| format!("No engine found for language: {}", lang_name))?;
                    all_mutations.extend(mutation_engine.get_mutations().iter().map(|m| {
                        Mutation {
                            slug: m.slug,
                            description: m.description,
                            severity: m.severity.clone(),
                        }
                    }));
                }
            }
        }
        let json_mutations = JsonMutations {
            mutations: all_mutations,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&json_mutations).map_err(|e| e.to_string())?
        );
    } else {
        // Table format
        match &language {
            Some(lang_str) => {
                print_mutations_for_language(lang_str, registry)?;
            }
            None => {
                // For each registered language, print its mutations
                for lang_name in registry.all_languages() {
                    print_mutations_for_language(lang_name, registry)?;
                }
            }
        };
    }

    Ok(())
}

fn is_move_language_name(language_name: &str) -> bool {
    language_name.eq_ignore_ascii_case("move")
        || language_name.eq_ignore_ascii_case("suimove")
        || language_name.eq_ignore_ascii_case("sui_move")
        || language_name.eq_ignore_ascii_case("move/sui")
        || language_name.eq_ignore_ascii_case("move:iota")
        || language_name.eq_ignore_ascii_case("move/iota")
        || language_name.eq_ignore_ascii_case("move:sui")
}

fn print_mutations_for_language(
    lang_name: &str,
    registry: &LanguageRegistry,
) -> Result<(), String> {
    let mutation_engine = registry
        .get_engine(lang_name)
        .ok_or_else(|| format!("No engine found for language: {}", lang_name))?;
    let mutations = mutation_engine.get_mutations();

    // Group mutations by slug
    let mut mutation_groups: HashMap<&str, (MutationSeverity, Vec<&str>)> = HashMap::new();

    for mutation in mutations {
        let entry = mutation_groups
            .entry(mutation.slug)
            .or_insert((mutation.severity.clone(), Vec::new()));
        entry.1.push(mutation.description);
    }

    // Sort slugs for consistent output
    let mut slugs: Vec<_> = mutation_groups.keys().copied().collect();
    slugs.sort();

    info!("Available mutations for {}:", lang_name);
    for slug in slugs {
        let (severity, descriptions) = &mutation_groups[slug];
        if descriptions.len() == 1 {
            info!("  [{}] {} (Severity: {})", slug, descriptions[0], severity);
        } else {
            info!(
                "  [{}] {} (and {} other variants) (Severity: {})",
                slug,
                descriptions[0],
                descriptions.len() - 1,
                severity
            );
        }
    }
    Ok(())
}
