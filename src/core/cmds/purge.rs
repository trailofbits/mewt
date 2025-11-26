use log::{error, info};
use std::io::{self, Write};
use std::path::PathBuf;

use indicatif::HumanDuration;
use std::time::Duration;

use crate::SqlStore;
use crate::core::cli::PurgeArgs;
use crate::types::{AppError, AppResult};

async fn get_target_id_by_path(store: &SqlStore, path: &str) -> AppResult<Option<i64>> {
    let targets = store.get_all_targets().await?;

    let normalized_path = PathBuf::from(path).canonicalize()?;
    for target in targets {
        if target.path == normalized_path {
            return Ok(Some(target.id));
        }
    }

    Ok(None)
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

pub async fn execute_purge(args: PurgeArgs, store: SqlStore) -> AppResult<()> {
    match args.target {
        Some(target_path) => {
            // Purge a specific target
            match get_target_id_by_path(&store, &target_path).await? {
                Some(target_id) => purge_target(&store, target_id, &target_path).await?,
                None => {
                    error!("Target not found: {target_path}");
                    return Err(AppError::TargetNotFound(target_path));
                }
            }
        }
        None => {
            // Purge all targets
            info!("Purging all targets...");
            let targets = store.get_all_targets().await?;

            if targets.is_empty() {
                info!("No targets found in database.");
                return Ok(());
            }

            info!("Found {} targets to purge", targets.len());

            for target in targets {
                let path_display = target.display();
                purge_target(&store, target.id, &path_display).await?;
            }
        }
    }

    info!("Purge complete");
    Ok(())
}
