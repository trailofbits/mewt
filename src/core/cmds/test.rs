use log::{error, info, warn};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::TestArgs;
use crate::core::runner::TestRunner;
use crate::types::AppResult;
use crate::types::config::{config, resolve_test_for_path_with_cli};

/// Read mutant IDs from --ids-file (file or stdin) or --ids (CLI arg).
/// --ids-file takes precedence over --ids.
fn read_mutant_ids(args: &TestArgs) -> io::Result<Vec<i64>> {
    let input = if let Some(ref path) = args.ids_file {
        // Read from file or stdin
        if path == "-" {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        } else {
            fs::read_to_string(path)?
        }
    } else if let Some(ref ids_str) = args.ids {
        // Use CLI --ids arg (comma-separated for backwards compatibility)
        ids_str.clone()
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Either --ids or --ids-file must be provided",
        ));
    };

    // Parse IDs from input (supports whitespace, newlines, and commas)
    let mut ids = Vec::new();
    for token in input.split(|c: char| c.is_whitespace() || c == ',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        match trimmed.parse::<i64>() {
            Ok(id) => ids.push(id),
            Err(_) => {
                warn!("Skipping invalid mutant ID: {}", trimmed);
            }
        }
    }

    Ok(ids)
}

pub async fn execute_test(
    args: TestArgs,
    store: SqlStore,
    running: Arc<AtomicBool>,
    registry: Arc<LanguageRegistry>,
) -> AppResult<()> {
    // Read IDs from file/stdin or CLI arg
    let ids = read_mutant_ids(&args)?;

    if ids.is_empty() {
        return Err(
            io::Error::new(io::ErrorKind::InvalidInput, "No valid mutant IDs provided").into(),
        );
    }

    info!("Testing mutants: {ids:?}");

    // Resolve test command per mutant's target and group by (cmd, timeout)
    let mut groups: HashMap<(String, Option<u32>), Vec<i64>> = HashMap::new();
    for id in ids {
        match store.get_mutant(id).await {
            Ok(mutant) => match store.get_target(mutant.target_id).await {
                Ok(target) => {
                    let (maybe_cmd, timeout) =
                        resolve_test_for_path_with_cli(&target.path, &args.test_cmd, args.timeout);
                    if let Some(cmd) = maybe_cmd {
                        groups.entry((cmd, timeout)).or_default().push(id);
                    } else {
                        warn!("No test command provided");
                    }
                }
                Err(e) => error!("Failed to get target for mutant {id}: {e}"),
            },
            Err(e) => error!("Failed to get mutant {id}: {e}"),
        }
    }

    // For each group, baseline once and test the group's mutants
    for ((cmd, timeout), group_ids) in groups.into_iter() {
        if !running.load(Ordering::SeqCst) {
            warn!("Testing interrupted, stopping...");
            break;
        }

        let mut runner = match TestRunner::new_with_baseline(
            cmd,
            timeout.or(config().test.timeout),
            Arc::clone(&running),
            store.clone(),
            false, // No need for comprehensive mode during targeted re-tests
            args.verbose,
            Arc::clone(&registry),
        )
        .await
        {
            Ok(runner) => runner,
            Err(e) => return Err(e.into()),
        };

        for id in group_ids {
            if !running.load(Ordering::SeqCst) {
                warn!("Testing interrupted, stopping...");
                break;
            }
            match store.get_mutant(id).await {
                Ok(mutant) => match store.get_target(mutant.target_id).await {
                    Ok(target) => {
                        info!("Testing mutant {} for target: {}", id, target.display());
                        let mut duration_ms = 0;
                        let result = runner.test_mutant(target, mutant, &mut duration_ms).await;
                        match result {
                            Ok(_) => info!("Mutant {id} tested successfully"),
                            Err(e) => error!("Failed to test mutant {id}: {e}"),
                        }
                    }
                    Err(e) => error!("Failed to get target for mutant {id}: {e}"),
                },
                Err(e) => error!("Failed to get mutant {id}: {e}"),
            }
        }
    }

    Ok(())
}
