use console::style;
use log::info;
use serde::Serialize;
use std::collections::BTreeMap;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cmds::print::MutantsFilters;
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
        || filters.mutation_type.is_some()
        || filters.tested
        || filters.untested;

    if use_filters {
        // Get filtered mutants from database
        let mut results = store
            .get_mutants_filtered(
                filters.target.clone(),
                filters.line,
                filters.mutation_type.clone(),
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
                info!("  {}", mutant.display(target));
            }
            info!(""); // Empty line between targets
        }

        return Ok(());
    }

    // Legacy path: no filters, use old logic with target filtering or config
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
                    info!("  {}", mutant.display(&target));
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
