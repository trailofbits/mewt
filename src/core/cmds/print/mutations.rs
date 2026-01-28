use std::collections::HashMap;

use log::info;
use serde::Serialize;

use crate::LanguageRegistry;
use crate::core::cmds::print::MutationsFilters;
use crate::types::{Mutation, MutationSeverity};

#[derive(Serialize)]
struct JsonMutations {
    mutations: Vec<Mutation>,
}

pub async fn execute(filters: MutationsFilters, registry: &LanguageRegistry) -> Result<(), String> {
    let language = filters.language;
    let is_json_format = filters.format == "json";
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
