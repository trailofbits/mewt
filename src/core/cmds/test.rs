use log::{error, info, warn};
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::TestArgs;
use crate::core::runner::TestRunner;
use crate::types::AppResult;
use crate::types::config::{config, resolve_test_for_path_with_cli};

pub async fn execute_test(
    args: TestArgs,
    store: SqlStore,
    running: Arc<AtomicBool>,
    registry: Arc<LanguageRegistry>,
) -> AppResult<()> {
    let ids: Vec<i64> = args
        .ids
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

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
