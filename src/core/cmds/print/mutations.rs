use std::collections::HashMap;
use std::path::Path;

use log::{info, warn};
use serde::Serialize;

use crate::LanguageRegistry;
use crate::core::cmds::print::MutationsFilters;
use crate::languages::r#move::dialect::is_move_language_name;
use crate::types::config::{ResolvedMoveDialect, config};
use crate::types::{Mutation, MutationSeverity};

#[derive(Serialize)]
struct JsonMutations {
    mutations: Vec<Mutation>,
}

pub async fn execute(filters: MutationsFilters, registry: &LanguageRegistry) -> Result<(), String> {
    let language = filters.language;
    let is_json_format = filters.format == "json";

    if filters.dialect.is_some() && !language.as_deref().is_some_and(is_move_language_name) {
        return Err(
            "--dialect requires --language move (or move/<dialect>) for `print mutations`"
                .to_string(),
        );
    }

    let need_move_dialect = language.as_deref().is_some_and(is_move_language_name);
    let resolved_move_dialect = if need_move_dialect {
        let resolved = config()
            .resolve_move_dialect(filters.dialect.as_deref())
            .map_err(|e| e.to_string())?;
        if resolved.defaulted {
            warn!(
                "Move dialect not explicitly set; defaulting to '{}'. Use --dialect or [languages.move].dialect to select sui|iota|aptos explicitly.",
                resolved.dialect.as_str()
            );
        } else {
            info!(
                "Using Move dialect '{}' for mutation listing",
                resolved.dialect.as_str()
            );
        }
        Some(resolved)
    } else {
        None
    };

    if is_json_format {
        let mut all_mutations = Vec::new();
        match &language {
            Some(lang_str) => {
                let (engine_name, _) =
                    resolve_language_for_print(registry, lang_str, resolved_move_dialect)?;
                let mutation_engine = registry
                    .get_engine(&engine_name)
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
        match &language {
            Some(lang_str) => {
                let (engine_name, display_name) =
                    resolve_language_for_print(registry, lang_str, resolved_move_dialect)?;
                print_mutations_for_language(&engine_name, &display_name, registry)?;
            }
            None => {
                for lang_name in registry.all_languages() {
                    let display_name = if is_move_language_name(lang_name) {
                        if let Some(resolved) = resolved_move_dialect {
                            registry
                                .resolve_selection_for_path(
                                    Path::new("__virtual__.move"),
                                    Some(lang_name),
                                    resolved,
                                )
                                .map(|selection| selection.canonical_label)
                                .unwrap_or_else(|_| lang_name.to_string())
                        } else {
                            lang_name.to_string()
                        }
                    } else {
                        lang_name.to_string()
                    };
                    print_mutations_for_language(lang_name, &display_name, registry)?;
                }
            }
        };
    }

    Ok(())
}

fn resolve_language_for_print(
    registry: &LanguageRegistry,
    raw_language: &str,
    resolved_move_dialect: Option<ResolvedMoveDialect>,
) -> Result<(String, String), String> {
    if is_move_language_name(raw_language) {
        let resolved = resolved_move_dialect
            .ok_or_else(|| "Move language selection requires resolved dialect".to_string())?;
        let selection = registry.resolve_selection_for_path(
            Path::new("__virtual__.move"),
            Some(raw_language),
            resolved,
        )?;
        return Ok((selection.language_key, selection.canonical_label));
    }

    let engine = registry
        .get_engine(raw_language)
        .ok_or_else(|| format!("No engine found for language: {}", raw_language))?;
    Ok((engine.name().to_string(), engine.name().to_string()))
}

fn print_mutations_for_language(
    engine_lookup_name: &str,
    display_name: &str,
    registry: &LanguageRegistry,
) -> Result<(), String> {
    let mutation_engine = registry
        .get_engine(engine_lookup_name)
        .ok_or_else(|| format!("No engine found for language: {}", engine_lookup_name))?;
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
