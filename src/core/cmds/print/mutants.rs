use console::style;
use log::info;
use serde::Serialize;

use crate::SqlStore;
use crate::core::cmds::print::MutantsFilters;
use crate::types::{AppResult, Mutant, Target};

#[derive(Serialize)]
struct JsonMutant {
    mutant: Mutant,
    target: Target,
}

#[derive(Serialize)]
struct JsonMutants {
    mutants: Vec<JsonMutant>,
}

pub async fn execute(store: SqlStore, filters: MutantsFilters) -> AppResult<()> {
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
        let results = store
            .get_mutants_filtered(
                filters.target.clone(),
                filters.line,
                filters.mutation_type.clone(),
                filters.tested,
                filters.untested,
            )
            .await?;

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

        // Group by target for display
        let mut by_target: std::collections::HashMap<i64, Vec<_>> =
            std::collections::HashMap::new();
        for (mutant, target) in results {
            by_target
                .entry(target.id)
                .or_insert_with(Vec::new)
                .push((mutant, target));
        }

        // Display grouped results
        for (_, entries) in by_target {
            if entries.is_empty() {
                continue;
            }
            let target = &entries[0].1;
            info!("{}", style(format!("Target: {}", target.display())).bold());

            for (mutant, target) in entries {
                info!("  {}", mutant.display(&target));
            }
            info!(""); // Empty line between targets
        }

        return Ok(());
    }

    // Legacy path: no filters, use old logic with target filtering
    let filtered_targets = Target::filter_by_path(&store, filters.target.clone()).await?;
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

    // Collect all mutants for JSON format
    if is_json_format {
        let mut all_mutants = Vec::new();
        for target in filtered_targets {
            let mutants = store.get_mutants(target.id).await?;
            for mutant in mutants {
                all_mutants.push(JsonMutant {
                    mutant,
                    target: target.clone(),
                });
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

        // Print mutants
        for mutant in mutants {
            if is_ids_format {
                info!("{}", mutant.id);
            } else {
                info!("  {}", mutant.display(&target));
            }
        }

        if !is_ids_format {
            info!(""); // Empty line between targets
        }
    }

    Ok(())
}
