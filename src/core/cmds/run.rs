use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{info, warn};
use std::collections::HashMap;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::RunArgs;
use crate::core::resolver::ResolutionDefaults;
use crate::core::runner::TestRunner;
use crate::core::utils::parse_csv;
use crate::types::config::{ResolvedTargets, config, resolve_test_for_path};
use crate::types::{AppResult, CampaignSummary, Target};

type RunGroupKey = (String, Option<u32>, Option<Vec<String>>, bool);

#[allow(clippy::too_many_arguments)]
pub async fn execute_run(
    args: RunArgs,
    store: SqlStore,
    running: Arc<AtomicBool>,
    registry: Arc<LanguageRegistry>,
    resolved_targets: Option<ResolvedTargets>,
    _mutations: Option<Vec<String>>,
    test_cmd: Option<String>,
    test_timeout: Option<u32>,
    resolution_defaults: ResolutionDefaults,
) -> AppResult<Option<CampaignSummary>> {
    let cli_mutations = parse_csv::<String>(args.mutations.as_deref());
    let cli_mutations_slice = cli_mutations.as_deref();

    let targets = if let Some(resolved) = resolved_targets {
        // Generate new mutants for the specified targets
        let targets =
            Target::load_targets(&resolved, &store, &registry, None, &resolution_defaults).await?;
        for target in targets.iter() {
            let (target_mutations, _) = config().resolve_run_for_path(
                &target.path,
                cli_mutations_slice,
                args.comprehensive,
            );
            let mutants_res = target.generate_mutants(&registry, target_mutations.as_deref());
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
            return Ok(None);
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

    // Group targets by resolved (test_cmd, timeout, mutations, comprehensive)
    let mut groups: HashMap<RunGroupKey, Vec<Target>> = HashMap::new();
    for target in targets.into_iter() {
        let (maybe_cmd, timeout) =
            resolve_test_for_path(&target.path, test_cmd.as_deref(), test_timeout);
        let (target_mutations, comprehensive) =
            config().resolve_run_for_path(&target.path, cli_mutations_slice, args.comprehensive);
        if let Some(cmd) = maybe_cmd {
            groups
                .entry((cmd, timeout, target_mutations, comprehensive))
                .or_default()
                .push(target);
        } else {
            warn!("No test command provided for target {}", target.display());
        }
    }

    // For each group, create a runner (baseline once per unique cmd) and run campaign
    for ((cmd, timeout, group_mutations, comprehensive), group_targets) in groups.into_iter() {
        // Targets are already sorted by path from load_targets()
        if !running.load(Ordering::SeqCst) {
            warn!("Mutation campaign cancelled before execution");
            break;
        }

        let mut runner = match TestRunner::new_with_baseline(
            cmd,
            timeout.or(config().test().timeout()),
            Arc::clone(&running),
            store.clone(),
            comprehensive,
            args.verbose,
            Arc::clone(&registry),
        )
        .await
        {
            Ok(runner) => runner,
            Err(e) => return Err(e.into()),
        };

        runner
            .run_mutation_campaign(group_targets, group_mutations.as_ref().map(|v| v.join(",")))
            .await?;
    }

    // Query DB once at the end for final counts
    let final_summary = store.get_campaign_summary().await?;
    Ok(Some(final_summary))
}
