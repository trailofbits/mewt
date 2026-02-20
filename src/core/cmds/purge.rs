use log::{error, info};
use std::io::{self, Write};
use std::path::PathBuf;

use indicatif::HumanDuration;
use std::time::Duration;

use crate::SqlStore;
use crate::core::cli::PurgeArgs;
use crate::types::config::config;
use crate::types::{AppError, AppResult, Target};

async fn get_target_ids_by_path(store: &SqlStore, path: &str) -> AppResult<Vec<i64>> {
    let targets = store.get_all_targets().await?;
    let mut matching_ids = Vec::new();

    // Check if the path contains glob characters
    if path.contains('*') || path.contains('?') || path.contains('[') {
        // Treat as glob pattern
        let glob_pattern = globset::Glob::new(path)
            .map_err(|e| AppError::Custom(format!("Invalid glob pattern '{}': {}", path, e)))?
            .compile_matcher();

        for target in targets {
            if glob_pattern.is_match(&target.path) {
                matching_ids.push(target.id);
            }
        }
    } else {
        // Exact path match (try canonicalization)
        match PathBuf::from(path).canonicalize() {
            Ok(normalized_path) => {
                for target in targets {
                    if target.path == normalized_path {
                        matching_ids.push(target.id);
                        break;
                    }
                }
            }
            Err(_) => {
                // If canonicalization fails, try non-canonical match
                let path_buf = PathBuf::from(path);
                for target in targets {
                    if target.path == path_buf {
                        matching_ids.push(target.id);
                        break;
                    }
                }
            }
        }
    }

    Ok(matching_ids)
}

/// Ask for user confirmation before proceeding
fn confirm_action(prompt: &str) -> AppResult<bool> {
    print!("{prompt} (y/n): ");
    io::stdout().flush().map_err(AppError::Io)?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(AppError::Io)?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

async fn purge_target(store: &SqlStore, target_id: i64, target_path: &str) -> AppResult<()> {
    // Get information about mutants and outcomes for this target
    let mutants = match store.get_mutants(target_id).await {
        Ok(mutants) => mutants,
        Err(e) => {
            error!("Failed to retrieve mutants for target {target_id}: {e}");
            return Err(AppError::Database(format!(
                "Failed to retrieve mutants: {e}"
            )));
        }
    };

    let outcomes = match store.get_outcomes(target_id).await {
        Ok(outcomes) => outcomes,
        Err(e) => {
            error!("Failed to retrieve outcomes for target {target_id}: {e}");
            return Err(AppError::Database(format!(
                "Failed to retrieve outcomes: {e}"
            )));
        }
    };

    // Calculate total runtime
    let total_duration_ms: u32 = outcomes.iter().map(|o| o.duration_ms).sum();

    // Ask for confirmation before proceeding
    let prompt = format!(
        "Are you sure you want to delete target '{}' and all associated mutants?\n\
         These {} mutants took {} of runtime to generate ({} have outcomes)",
        target_path,
        mutants.len(),
        HumanDuration(Duration::from_millis(total_duration_ms as u64)),
        outcomes.len()
    );

    if !confirm_action(&prompt)? {
        info!("Skipping target: {target_path}");
        return Ok(());
    }
    info!("Purging target: {target_path} (ID: {target_id})");

    match store.remove_target(target_id).await {
        Ok(_) => info!("Removed target {target_id} and all associated mutants and outcomes"),
        Err(e) => {
            error!("Failed to remove target {target_id}: {e}");
            return Err(AppError::Database(format!("Failed to remove target: {e}")));
        }
    }

    Ok(())
}

/// Get targets that should be purged by default (not in config or ignored)
async fn get_default_purge_targets(store: &SqlStore) -> AppResult<Vec<(i64, String)>> {
    // Get all targets from database
    let all_targets = store.get_all_targets().await?;

    // Get config targets (if any)
    let targets_config = config().targets();

    if let Some(targets_cfg) = targets_config {
        if let Some(include_patterns) = &targets_cfg.include {
            // Get the "active" targets using config patterns
            let resolved = crate::types::config::ResolvedTargets {
                include: include_patterns.clone(),
                ignore: targets_cfg.ignore.clone().unwrap_or_default(),
            };

            let active_targets = Target::filter_existing_by_patterns(store, &resolved)
                .await
                .map_err(|e| AppError::Custom(format!("Failed to filter targets: {}", e)))?;

            // Build a set of active target IDs for quick lookup
            let active_ids: std::collections::HashSet<i64> =
                active_targets.iter().map(|t| t.id).collect();

            // Purge targets NOT in the active set
            let mut targets_to_purge = Vec::new();
            for target in all_targets {
                if !active_ids.contains(&target.id) {
                    targets_to_purge.push((target.id, target.display()));
                }
            }

            return Ok(targets_to_purge);
        }
    }

    // No config targets defined - don't purge anything by default
    info!(
        "No config [targets] defined. Use --all to purge all targets or --target to specify targets."
    );
    Ok(vec![])
}

pub async fn execute_purge(args: PurgeArgs, store: SqlStore) -> AppResult<()> {
    let targets_to_purge: Vec<(i64, String)> = if args.all {
        // Purge all targets
        info!("Purging all targets...");
        let targets = store.get_all_targets().await?;
        targets.iter().map(|t| (t.id, t.display())).collect()
    } else if let Some(target_path) = args.target {
        // Purge specific target(s) matching the path/glob
        let matching_ids = get_target_ids_by_path(&store, &target_path).await?;

        if matching_ids.is_empty() {
            error!("No targets found matching: {target_path}");
            return Err(AppError::Custom(format!(
                "No targets found matching pattern: {}",
                target_path
            )));
        }

        // Get display paths for matched targets
        let mut result = Vec::new();
        for id in matching_ids {
            if let Ok(target) = store.get_target(id).await {
                result.push((id, target.display()));
            }
        }
        result
    } else {
        // Default behavior: purge targets not in config or ignored
        info!("Purging targets not in config [targets].include or in [targets].ignore...");
        get_default_purge_targets(&store).await?
    };

    if targets_to_purge.is_empty() {
        info!("No targets found to purge.");
        return Ok(());
    }

    info!("Found {} target(s) to purge", targets_to_purge.len());

    for (target_id, path_display) in targets_to_purge {
        purge_target(&store, target_id, &path_display).await?;
    }

    info!("Purge complete");
    Ok(())
}
