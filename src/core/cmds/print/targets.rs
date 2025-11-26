use std::fs;

use console::style;
use log::info;

use crate::SqlStore;
use crate::types::{AppResult, Hash};

pub async fn execute(store: SqlStore) -> AppResult<()> {
    // Get all targets
    let targets = store.get_all_targets().await?;
    if targets.is_empty() {
        info!("No targets found");
        return Ok(());
    }

    for target in targets {
        info!("Target: {} (ID: {})", target.display(), target.id);

        // Check if the file still exists and compute its current hash
        let file_status = if target.path.exists() {
            match fs::read_to_string(&target.path) {
                Ok(content) => {
                    let current_hash = Hash::digest(content);
                    if current_hash == target.file_hash {
                        style("match").green()
                    } else {
                        style("modified").yellow()
                    }
                }
                Err(_) => style("error reading").red(),
            }
        } else {
            style("no longer exists").red()
        };

        info!("  Hash: {} ({})", target.file_hash.to_hex(), file_status);
        info!(""); // Empty line between targets
    }

    Ok(())
}
