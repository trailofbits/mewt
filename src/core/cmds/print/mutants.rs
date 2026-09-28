use console::style;
use log::info;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cmds::print::MutantsFilters;
use crate::core::prioritize::{self, Annotation, Purpose};
use crate::core::utils::parse_csv;
use crate::types::{AppResult, Mutant, MutationSeverity, Target};

#[derive(Serialize)]
struct JsonMutant {
    mutant: Mutant,
    target: Target,
}

#[derive(Serialize)]
struct JsonMutants {
    mutants: Vec<JsonMutant>,
}

async fn cached_pre_priorities(store: &SqlStore, targets: &[Target]) -> HashMap<i64, Annotation> {
    let mut priorities = HashMap::new();
    for target in targets {
        match prioritize::prepare(store, target, Purpose::Pre).await {
            Ok(plan) => match prioritize::annotations(store, target, &plan).await {
                Ok(found) => priorities.extend(found),
                Err(error) => log::warn!(
                    "Could not read cached priorities for {}: {error}",
                    target.display()
                ),
            },
            Err(error) => log::warn!(
                "Could not prepare cached priorities for {}: {error}",
                target.display()
            ),
        }
    }
    priorities
}

fn priority_label(priority: &Annotation, verbose: bool) -> String {
    if verbose {
        format!(
            "P={:.2}/4 (distribution confidence {:.2}; execution-value heuristic)",
            priority.score, priority.confidence
        )
    } else {
        format!("P={}", priority.score.round() as u8)
    }
}

fn display_mutant(
    mutant: &Mutant,
    target: &Target,
    priority: Option<&Annotation>,
    verbose: bool,
) -> String {
    let display = mutant.display(target);
    let Some(priority) = priority else {
        return display;
    };
    let prefix = format!("[{} {}]", mutant.mutation_slug, mutant.id);
    if let Some(rest) = display.strip_prefix(&prefix) {
        format!("{prefix} ({}){rest}", priority_label(priority, verbose))
    } else {
        // Keep the mutant visible if its display format ever changes.
        display
    }
}

pub async fn execute(
    store: SqlStore,
    filters: MutantsFilters,
    registry: &LanguageRegistry,
) -> AppResult<()> {
    // Handle format output
    let is_ids_format = filters.format == "ids";
    let is_json_format = filters.format == "json";

    // Use filtered query if any filters are provided
    let use_filters = filters.target.is_some()
        || filters.line.is_some()
        || filters.mutation_types.is_some()
        || filters.tested
        || filters.untested;

    // Parse mutation types CSV
    let mutation_slugs = parse_csv::<String>(filters.mutation_types.as_deref());

    if use_filters {
        // Get filtered mutants from database
        let mut results = store
            .get_mutants_filtered(
                filters.target.clone(),
                filters.line,
                mutation_slugs,
                filters.tested,
                filters.untested,
            )
            .await?;

        // Apply severity filter if provided (application-layer filtering)
        if let Some(severities) = parse_csv::<MutationSeverity>(filters.severity.as_deref()) {
            results.retain(|(mutant, target)| {
                if let Some(mutation) =
                    registry.get_mutation(&target.language, &mutant.mutation_slug)
                {
                    severities.contains(&mutation.severity)
                } else {
                    false // Filter out unknown mutations
                }
            });
        }

        if results.is_empty() {
            if is_json_format {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&JsonMutants { mutants: vec![] })?
                );
            } else if !is_ids_format {
                info!("No mutants found matching the filters");
            }
            return Ok(());
        }

        if is_json_format {
            let json_mutants = JsonMutants {
                mutants: results
                    .into_iter()
                    .map(|(mutant, target)| JsonMutant { mutant, target })
                    .collect(),
            };
            println!("{}", serde_json::to_string_pretty(&json_mutants)?);
            return Ok(());
        }

        if is_ids_format {
            // Just print IDs, one per line
            for (mutant, _) in results {
                info!("{}", mutant.id);
            }
            return Ok(());
        }

        let selected_targets: Vec<Target> = results
            .iter()
            .map(|(_, target)| target.clone())
            .fold(BTreeMap::new(), |mut targets, target| {
                targets.insert(target.id, target);
                targets
            })
            .into_values()
            .collect();
        let priorities = cached_pre_priorities(&store, &selected_targets).await;

        // Group by target path for display
        // Note: Data is already sorted by path from database query,
        // BTreeMap maintains this order since we insert in sorted order
        let mut by_target: BTreeMap<String, Vec<(Mutant, Target)>> = BTreeMap::new();
        for (mutant, target) in results {
            let path_key = target.path.to_string_lossy().to_string();
            by_target
                .entry(path_key)
                .or_default()
                .push((mutant, target));
        }

        // Display grouped results
        for entries in by_target.values() {
            if entries.is_empty() {
                continue;
            }
            let target = &entries[0].1;
            info!("{}", style(format!("Target: {}", target.display())).bold());

            for (mutant, target) in entries {
                info!(
                    "  {}",
                    display_mutant(mutant, target, priorities.get(&mutant.id), filters.verbose)
                );
            }
            info!(""); // Empty line between targets
        }

        return Ok(());
    }

    // Simple path: no filters, use target filtering or config
    let filtered_targets = Target::filter_by_path_or_config(&store, filters.target.clone()).await?;

    if filtered_targets.is_empty() {
        if is_json_format {
            println!(
                "{}",
                serde_json::to_string_pretty(&JsonMutants { mutants: vec![] })?
            );
        } else if !is_ids_format {
            info!("No targets found");
        }
        return Ok(());
    }

    // Parse severity filter once
    let severity_filter = parse_csv::<MutationSeverity>(filters.severity.as_deref());

    // Collect all mutants for JSON format
    if is_json_format {
        let mut all_mutants = Vec::new();
        for target in filtered_targets {
            let mutants = store.get_mutants(target.id).await?;
            for mutant in mutants {
                // Apply severity filter if provided
                let include = if let Some(ref severities) = severity_filter {
                    if let Some(mutation) =
                        registry.get_mutation(&target.language, &mutant.mutation_slug)
                    {
                        severities.contains(&mutation.severity)
                    } else {
                        false
                    }
                } else {
                    true
                };

                if include {
                    all_mutants.push(JsonMutant {
                        mutant,
                        target: target.clone(),
                    });
                }
            }
        }
        let json_mutants = JsonMutants {
            mutants: all_mutants,
        };
        println!("{}", serde_json::to_string_pretty(&json_mutants)?);
        return Ok(());
    }

    // Group mutants by target
    for target in filtered_targets {
        if !is_ids_format {
            info!("{}", style(format!("Target: {}", target.display())).bold());
        }

        // Get all mutants for this target
        let mutants = store.get_mutants(target.id).await?;
        let priorities = if !is_ids_format {
            cached_pre_priorities(&store, std::slice::from_ref(&target)).await
        } else {
            HashMap::new()
        };
        if mutants.is_empty() {
            if !is_ids_format {
                info!("  No mutants found for this target");
            }
            continue;
        }

        // Print mutants (with severity filtering)
        let mut printed_any = false;
        for mutant in mutants {
            // Apply severity filter if provided
            let include = if let Some(ref severities) = severity_filter {
                if let Some(mutation) =
                    registry.get_mutation(&target.language, &mutant.mutation_slug)
                {
                    severities.contains(&mutation.severity)
                } else {
                    false
                }
            } else {
                true
            };

            if include {
                printed_any = true;
                if is_ids_format {
                    info!("{}", mutant.id);
                } else {
                    info!(
                        "  {}",
                        display_mutant(
                            &mutant,
                            &target,
                            priorities.get(&mutant.id),
                            filters.verbose
                        )
                    );
                }
            }
        }

        if !is_ids_format && !printed_any {
            info!("  No mutants found for this target (after filtering)");
        }

        if !is_ids_format {
            info!(""); // Empty line between targets
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Hash;
    use std::path::PathBuf;

    #[test]
    fn cached_priority_sits_beside_id_and_rounds_only_in_compact_mode() {
        let source = "fn f() -> bool { true }";
        let target = Target {
            id: 1,
            path: PathBuf::from("example.rs"),
            file_hash: Hash::digest(source.into()),
            text: source.into(),
            language: "rust".parse().unwrap(),
        };
        let mutant = Mutant {
            id: 42,
            target_id: 1,
            mutation_slug: "BL".into(),
            byte_offset: 17,
            line_offset: 0,
            old_text: "true".into(),
            new_text: "false".into(),
        };
        let priority = Annotation {
            score: 0.64,
            confidence: 0.47,
            category: None,
        };
        let plain = display_mutant(&mutant, &target, None, false);
        assert_eq!(plain, mutant.display(&target));
        let compact = display_mutant(&mutant, &target, Some(&priority), false);
        assert!(compact.starts_with("[BL 42] (P=1) Line 1:"), "{compact}");
        assert!(compact.ends_with(&plain["[BL 42]".len()..]));
        let verbose = display_mutant(&mutant, &target, Some(&priority), true);
        assert!(verbose.starts_with("[BL 42] (P=0.64/4 (distribution confidence 0.47; execution-value heuristic)) Line 1:"), "{verbose}");
    }
}
