use std::collections::HashMap;

use log::info;

use crate::LanguageRegistry;
use crate::types::MutationSeverity;

pub async fn execute(language: Option<String>, registry: &LanguageRegistry) -> Result<(), String> {
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
    info!("");
    Ok(())
}
