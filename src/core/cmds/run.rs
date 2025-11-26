use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{info, warn};
use std::collections::HashMap;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::RunArgs;
use crate::core::runner::TestRunner;
use crate::types::config::{config, resolve_test_for_path_with_cli};
use crate::types::{AppResult, Target};

pub async fn execute_run(
    args: RunArgs,
    store: SqlStore,
    running: Arc<AtomicBool>,
    registry: Arc<LanguageRegistry>,
) -> AppResult<()> {
    let targets = if let Some(target_path) = &args.target {
        // Generate new mutants for the specified target
        let target = PathBuf::from(target_path)
            .canonicalize()
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("Invalid target: {e}")))?;

        let targets = Target::load_targets(target, &store, &registry).await?;
        for target in targets.iter() {
            let mutants_res = target.generate_mutants(&registry);
            if let Ok(mutants) = mutants_res {
                for mut mutant in mutants {
                    let new_id = store
                        .add_mutant(mutant.clone())
                        .await
                        .expect("failed to add mutant");
                    if let Some(id) = new_id {
                        mutant.id = id;
                        info!("  Saved new mutant: {}", mutant.display(target));
                    }
                }
            }
        }
        targets
    } else {
        // Skip mutation generation, get targets for existing mutants to test (no outcomes + timeouts)
        let (mutants_to_test, _, _) = store.get_mutants_to_test().await?;
        if mutants_to_test.is_empty() {
            info!("No mutants to test found in database");
            return Ok(());
        }

        // Get unique targets for these mutants
        let mut target_ids: Vec<i64> = mutants_to_test.iter().map(|m| m.target_id).collect();
        target_ids.sort_unstable();
        target_ids.dedup();

        let mut targets = Vec::new();
        for target_id in target_ids {
            targets.push(store.get_target(target_id).await?);
        }
        targets
    };

    // Group targets by resolved (test_cmd, timeout)
    let mut groups: HashMap<(String, Option<u32>), Vec<Target>> = HashMap::new();
    for target in targets.into_iter() {
        let (maybe_cmd, timeout) =
            resolve_test_for_path_with_cli(&target.path, &args.test_cmd, args.timeout);
        if let Some(cmd) = maybe_cmd {
            groups.entry((cmd, timeout)).or_default().push(target);
        } else {
            warn!("No test command provided for target {}", target.display());
        }
    }

    // For each group, create a runner (baseline once per unique cmd) and run campaign
    for ((cmd, timeout), group_targets) in groups.into_iter() {
        if !running.load(Ordering::SeqCst) {
            warn!("Mutation campaign cancelled before execution");
            break;
        }

        let mut runner = match TestRunner::new_with_baseline(
            cmd,
            timeout.or(config().test.timeout),
            Arc::clone(&running),
            store.clone(),
            args.comprehensive,
            args.verbose,
            Arc::clone(&registry),
        )
        .await
        {
            Ok(runner) => runner,
            Err(e) => return Err(e.into()),
        };

        runner
            .run_mutation_campaign(group_targets, args.mutations.clone())
            .await?;
    }

    Ok(())
}
