use log::info;
use serde::Serialize;
use std::collections::HashMap;

use crate::LanguageRegistry;
use crate::core::cmds::print::MutationsFilters;
use crate::core::resolver::ResolutionDefaults;
use crate::types::config::config;
use crate::types::{Mutation, MutationSeverity};

#[derive(Serialize)]
struct JsonMutations {
    mutations: Vec<Mutation>,
}

pub async fn execute(filters: MutationsFilters, registry: &LanguageRegistry) -> Result<(), String> {
    let language = filters.language;
    let is_json_format = filters.format == "json";

    if filters.dialect.is_some()
        && !language
            .as_deref()
            .is_some_and(|lang| registry.language_supports_cli_dialect_flag(lang))
    {
        return Err(
            "--dialect requires a --language value whose resolver accepts CLI dialects".to_string(),
        );
    }

    if filters.dialect.is_some()
        && language
            .as_deref()
            .is_some_and(language_label_includes_dialect)
    {
        return Err(
            "Use either a dialect-qualified --language or --language with --dialect, not both"
                .to_string(),
        );
    }

    let needs_cli_dialect_defaults = language.as_deref().is_some_and(|lang| {
        registry.language_supports_cli_dialect_flag(lang) && !language_label_includes_dialect(lang)
    });
    let resolution_defaults = if needs_cli_dialect_defaults {
        let cli_dialect_family = if filters.dialect.is_some() {
            registry.cli_dialect_family().map_err(|e| e.to_string())?
        } else {
            None
        };
        Some(
            config()
                .resolve_language_defaults(cli_dialect_family, filters.dialect.as_deref())
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    if is_json_format {
        let mut all_mutations = Vec::new();
        match &language {
            Some(lang_str) => {
                let (engine_name, _display_name) = resolve_language_for_print(
                    registry,
                    lang_str,
                    filters.dialect.as_deref(),
                    resolution_defaults.as_ref(),
                )?;
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
                let (engine_name, display_name) = resolve_language_for_print(
                    registry,
                    lang_str,
                    filters.dialect.as_deref(),
                    resolution_defaults.as_ref(),
                )?;
                print_mutations_for_language(&engine_name, &display_name, registry)?;
            }
            None => {
                for lang_name in registry.all_languages() {
                    let display_name = resolve_language_for_print(
                        registry,
                        lang_name,
                        filters.dialect.as_deref(),
                        resolution_defaults.as_ref(),
                    )
                    .map(|(_, display)| display)
                    .unwrap_or_else(|_| lang_name.to_string());
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
    explicit_dialect: Option<&str>,
    defaults: Option<&ResolutionDefaults>,
) -> Result<(String, String), String> {
    let canonical =
        registry.resolve_canonical_for_language_label(raw_language, explicit_dialect, defaults)?;
    Ok((canonical.clone(), canonical))
}

fn language_label_includes_dialect(raw_language: &str) -> bool {
    raw_language.contains('/') || raw_language.contains(':')
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
