use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use console::style;
use log::info;
use serde::Serialize;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::config::path_is_included;
use crate::types::{AppResult, Hash};

#[derive(Serialize)]
struct TargetRow {
    in_db: bool,
    on_disk: bool,
    included: bool,
    hash: String,
    path: String,
    mutants: Option<i64>,
}

#[derive(Serialize)]
struct JsonTargets {
    targets: Vec<TargetRow>,
}

pub async fn execute(
    store: SqlStore,
    format: String,
    registry: &LanguageRegistry,
) -> AppResult<()> {
    let is_json_format = format == "json";

    // Gather all unique (path, hash) combinations
    let rows = gather_all_target_rows(&store, registry).await?;

    if rows.is_empty() {
        if is_json_format {
            println!(
                "{}",
                serde_json::to_string_pretty(&JsonTargets { targets: vec![] })?
            );
        } else {
            info!("No targets found in database or filesystem");
        }
        return Ok(());
    }

    if is_json_format {
        let json_targets = JsonTargets { targets: rows };
        println!("{}", serde_json::to_string_pretty(&json_targets)?);
    } else {
        print_table(&rows);
    }

    Ok(())
}

/// Gather all unique (path, hash) combinations from both DB and filesystem
async fn gather_all_target_rows(
    store: &SqlStore,
    registry: &LanguageRegistry,
) -> AppResult<Vec<TargetRow>> {
    // Use a composite string key: "path|hash_hex"
    let mut rows_map: HashMap<String, TargetRow> = HashMap::new();

    // 1. Gather from database
    let db_targets = store.get_all_targets().await?;
    for target in &db_targets {
        let mutant_count = count_mutants_for_target(store, target.id).await?;

        let on_disk = check_file_hash_matches(&target.path, &target.file_hash);
        let included = path_is_included(&target.path);

        let key = format!(
            "{}|{}",
            target.path.to_string_lossy(),
            target.file_hash.to_hex()
        );

        rows_map.insert(
            key,
            TargetRow {
                in_db: true,
                on_disk,
                included,
                hash: target.file_hash.to_hex()[..8].to_string(),
                path: target.display(),
                mutants: Some(mutant_count),
            },
        );
    }

    // 2. Gather from filesystem (files that match include patterns)
    if let Some(fs_files) = gather_filesystem_targets(registry).await {
        for (path, hash) in fs_files {
            let key = format!("{}|{}", path.to_string_lossy(), hash.to_hex());

            // Skip if we already have this (path, hash) from DB
            if rows_map.contains_key(&key) {
                continue;
            }

            let included = path_is_included(&path);

            // Get relative path for display
            let display_path = get_display_path(&path);

            rows_map.insert(
                key,
                TargetRow {
                    in_db: false,
                    on_disk: true,
                    included,
                    hash: hash.to_hex()[..8].to_string(),
                    path: display_path,
                    mutants: None,
                },
            );
        }
    }

    // 3. Sort rows: by path, then by in_db (true before false)
    let mut rows: Vec<TargetRow> = rows_map.into_values().collect();
    rows.sort_by(|a, b| {
        a.path.cmp(&b.path).then_with(|| b.in_db.cmp(&a.in_db)) // in_db=true comes first
    });

    Ok(rows)
}

/// Count mutants for a target
async fn count_mutants_for_target(store: &SqlStore, target_id: i64) -> AppResult<i64> {
    let mutants = store.get_mutants(target_id).await?;
    Ok(mutants.len() as i64)
}

/// Check if file at path exists and has the given hash
fn check_file_hash_matches(path: &PathBuf, expected_hash: &Hash) -> bool {
    if !path.exists() {
        return false;
    }
    match fs::read_to_string(path) {
        Ok(content) => {
            let current_hash = Hash::digest(content);
            current_hash == *expected_hash
        }
        Err(_) => false,
    }
}

/// Gather all files from filesystem that would be included by current config
async fn gather_filesystem_targets(registry: &LanguageRegistry) -> Option<Vec<(PathBuf, Hash)>> {
    use crate::types::config::config;

    let targets_cfg = config().targets()?;
    let include_patterns = targets_cfg.include.as_ref()?;
    let ignore_patterns = targets_cfg.ignore.as_deref().unwrap_or(&[]);

    let mut files = Vec::new();

    for pattern in include_patterns {
        let path = PathBuf::from(pattern);

        if path.is_file() {
            if !is_path_excluded_local(&path, ignore_patterns) {
                if registry.language_from_path(&path).is_some() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let hash = Hash::digest(content);
                        files.push((path, hash));
                    }
                }
            }
        } else if path.is_dir() {
            if let Ok(dir_files) = walk_directory(&path, ignore_patterns, registry) {
                files.extend(dir_files);
            }
        } else {
            // Try as glob
            if let Ok(paths) = glob::glob(pattern) {
                for entry in paths.flatten() {
                    if entry.is_file() && !is_path_excluded_local(&entry, ignore_patterns) {
                        if registry.language_from_path(&entry).is_some() {
                            if let Ok(content) = fs::read_to_string(&entry) {
                                let hash = Hash::digest(content);
                                files.push((entry, hash));
                            }
                        }
                    } else if entry.is_dir() {
                        if let Ok(dir_files) = walk_directory(&entry, ignore_patterns, registry) {
                            files.extend(dir_files);
                        }
                    }
                }
            }
        }
    }

    Some(files)
}

/// Walk a directory recursively, collecting files
fn walk_directory(
    dir: &PathBuf,
    ignore_patterns: &[String],
    registry: &LanguageRegistry,
) -> std::io::Result<Vec<(PathBuf, Hash)>> {
    let mut files = Vec::new();

    if is_path_excluded_local(dir, ignore_patterns) {
        return Ok(files);
    }

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if !is_path_excluded_local(&path, ignore_patterns) {
                if registry.language_from_path(&path).is_some() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let hash = Hash::digest(content);
                        files.push((path, hash));
                    }
                }
            }
        } else if path.is_dir() {
            files.extend(walk_directory(&path, ignore_patterns, registry)?);
        }
    }

    Ok(files)
}

/// Local copy to avoid circular dependency
fn is_path_excluded_local(path: &std::path::Path, ignore_patterns: &[String]) -> bool {
    if ignore_patterns.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    ignore_patterns
        .iter()
        .filter(|p| !p.is_empty())
        .any(|pat| path_str.contains(pat))
}

/// Get display path (relative to cwd)
fn get_display_path(path: &PathBuf) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        let target_abs = if path.is_absolute() {
            path.clone()
        } else {
            cwd.join(path)
        };

        if let Ok(relative) = target_abs.strip_prefix(&cwd) {
            let s = relative.to_string_lossy().to_string();
            if s.is_empty() { ".".to_string() } else { s }
        } else {
            path.to_string_lossy().to_string()
        }
    } else {
        path.to_string_lossy().to_string()
    }
}

/// Print the table with color coding
fn print_table(rows: &[TargetRow]) {
    // Print header
    println!(
        "{:5} | {:7} | {:8} | {:7} | {:8} | {}",
        style("In DB").bold(),
        style("On Disk").bold(),
        style("Included").bold(),
        style("Mutants").bold(),
        style("Hash").bold(),
        style("Path").bold()
    );
    println!("{}", style("-".repeat(90)).dim());

    // Print rows with color coding
    for row in rows {
        let in_db_str = if row.in_db { "Yes" } else { "No" };
        let on_disk_str = if row.on_disk { "Yes" } else { "No" };
        let included_str = if row.included { "Yes" } else { "No" };
        let mutants_str = row
            .mutants
            .map(|m| m.to_string())
            .unwrap_or_else(|| "-".to_string());

        // Color coding based on state
        let colored_line = match (row.in_db, row.on_disk, row.included) {
            (true, true, true) => {
                // Green: perfect state
                style(format!(
                    "{:5} | {:7} | {:8} | {:7} | {:8} | {}",
                    in_db_str, on_disk_str, included_str, mutants_str, row.hash, row.path
                ))
                .green()
            }
            (true, false, true) => {
                // Red: deleted from disk
                style(format!(
                    "{:5} | {:7} | {:8} | {:7} | {:8} | {}",
                    in_db_str, on_disk_str, included_str, mutants_str, row.hash, row.path
                ))
                .red()
            }
            (true, false, false) => {
                // Red: orphaned, safe to purge
                style(format!(
                    "{:5} | {:7} | {:8} | {:7} | {:8} | {}",
                    in_db_str, on_disk_str, included_str, mutants_str, row.hash, row.path
                ))
                .red()
            }
            (false, true, true) => {
                // Yellow: new version, ready to mutate
                style(format!(
                    "{:5} | {:7} | {:8} | {:7} | {:8} | {}",
                    in_db_str, on_disk_str, included_str, mutants_str, row.hash, row.path
                ))
                .yellow()
            }
            (_, _, false) => {
                // Gray/Dim: excluded by config
                style(format!(
                    "{:5} | {:7} | {:8} | {:7} | {:8} | {}",
                    in_db_str, on_disk_str, included_str, mutants_str, row.hash, row.path
                ))
                .dim()
            }
            _ => {
                // Default: no special color
                style(format!(
                    "{:5} | {:7} | {:8} | {:7} | {:8} | {}",
                    in_db_str, on_disk_str, included_str, mutants_str, row.hash, row.path
                ))
            }
        };

        println!("{}", colored_line);
    }
}
