use log::info;
use serde::Serialize;
use std::collections::HashMap;

use crate::LanguageRegistry;
use crate::core::cmds::print::MutationsFilters;
use crate::types::{Language, Mutation, MutationSeverity};

#[derive(Serialize)]
struct JsonMutations {
    mutations: Vec<Mutation>,
}

pub async fn execute(filters: MutationsFilters, registry: &LanguageRegistry) -> Result<(), String> {
    let language = filters.language;
    let is_json_format = filters.format == "json";

    if is_json_format {
        let mut all_mutations = Vec::new();
        for engine_language in languages_for_print(registry, language.as_deref())? {
            let mutation_engine = registry
                .get_engine(&engine_language)
                .ok_or_else(|| format!("No engine found for language: {engine_language}"))?;
            all_mutations.extend(mutation_engine.get_mutations().iter().map(|m| Mutation {
                slug: m.slug,
                description: m.description,
                severity: m.severity.clone(),
            }));
        }
        let json_mutations = JsonMutations {
            mutations: all_mutations,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&json_mutations).map_err(|e| e.to_string())?
        );
    } else {
        for engine_language in languages_for_print(registry, language.as_deref())? {
            let display_name = engine_language.to_string();
            print_mutations_for_language(&engine_language, &display_name, registry)?;
        }
    }

    Ok(())
}

fn languages_for_print(
    registry: &LanguageRegistry,
    raw_language: Option<&str>,
) -> Result<Vec<Language>, String> {
    let Some(raw_language) = raw_language else {
        return Ok(registry.all_languages());
    };

    registry
        .filter_labels(raw_language)
        .into_iter()
        .map(|label| registry.resolve_canonical_for_language_label(&label, None))
        .collect()
}

fn print_mutations_for_language(
    engine_language: &Language,
    display_name: &str,
    registry: &LanguageRegistry,
) -> Result<(), String> {
    let mutation_engine = registry
        .get_engine(engine_language)
        .ok_or_else(|| format!("No engine found for language: {}", engine_language))?;
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

    info!("Available mutations for {}:", display_name);
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
