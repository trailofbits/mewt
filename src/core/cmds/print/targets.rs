use std::fs;

use console::style;
use log::info;
use serde::Serialize;

use crate::SqlStore;
use crate::types::{AppResult, Hash, Target};

#[derive(Serialize)]
struct TargetInfo {
    target: Target,
    file_status: String,
}

#[derive(Serialize)]
struct JsonTargets {
    targets: Vec<TargetInfo>,
}

pub async fn execute(store: SqlStore, format: String) -> AppResult<()> {
    let is_json_format = format == "json";
    // Get all targets
    let targets = store.get_all_targets().await?;
    if targets.is_empty() {
        if is_json_format {
            println!(
                "{}",
                serde_json::to_string_pretty(&JsonTargets { targets: vec![] })?
            );
        } else {
            info!("No targets found");
        }
        return Ok(());
    }

    if is_json_format {
        let target_infos: Vec<TargetInfo> = targets
            .into_iter()
            .map(|target| {
                let file_status = if target.path.exists() {
                    match fs::read_to_string(&target.path) {
                        Ok(content) => {
                            let current_hash = Hash::digest(content);
                            if current_hash == target.file_hash {
                                "match".to_string()
                            } else {
                                "modified".to_string()
                            }
                        }
                        Err(_) => "error reading".to_string(),
                    }
                } else {
                    "no longer exists".to_string()
                };
                TargetInfo {
                    target,
                    file_status,
                }
            })
            .collect();
        let json_targets = JsonTargets {
            targets: target_infos,
        };
        println!("{}", serde_json::to_string_pretty(&json_targets)?);
    } else {
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
    }

    Ok(())
}
