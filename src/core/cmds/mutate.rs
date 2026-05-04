use log::{error, info, warn};
use std::sync::Arc;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::MutateArgs;
use crate::languages::r#move::dialect::is_move_language_name;
use crate::types::config::{ResolvedMoveDialect, ResolvedTargets};
use crate::types::{AppResult, MutationSeverity, Target};

pub async fn execute_mutate(
    args: MutateArgs,
    store: SqlStore,
    registry: Arc<LanguageRegistry>,
    resolved_targets: ResolvedTargets,
    mutations: Option<Vec<String>>,
    resolved_move_dialect: ResolvedMoveDialect,
) -> AppResult<()> {
    info!(
        "Generating mutants for targets: {:?}",
        resolved_targets.include
    );

    let mutations_slice = mutations.as_deref();

    // Load targets from the resolved configuration
    let targets = Target::load_targets(
        &resolved_targets,
        &store,
        &registry,
        mutations_slice,
        resolved_move_dialect,
    )
    .await?;

    let has_move_targets = targets
        .iter()
        .any(|target| is_move_language_name(&target.language));
    if has_move_targets {
        if resolved_move_dialect.defaulted {
            warn!(
                "Move dialect not explicitly set; defaulting to '{}'. Use --dialect or [languages.move].dialect to select sui|iota|auto explicitly.",
                resolved_move_dialect.dialect.as_str()
            );
        } else {
            info!(
                "Using Move dialect '{}' for .move targets",
                resolved_move_dialect.dialect.as_str()
            );
        }
    }

    let mut total_mutants = 0;
    let mut total_new_mutants = 0;

    // Generate and save mutants for each target
    for target in targets.iter() {
        let mutants_res = target.generate_mutants(&registry, mutations_slice);
        if let Ok(mutants) = mutants_res {
            let mutant_count = mutants.len();
            total_mutants += mutant_count;

            // Count mutants by severity (for all generated mutants, not just new ones)
            let mut high_count = 0;
            let mut medium_count = 0;
            let mut low_count = 0;

            for mutant in mutants {
                // Count by severity for summary (regardless of whether it's new)
                let severity = registry.get_severity(&target.language, &mutant.mutation_slug);
                match severity {
                    MutationSeverity::High => high_count += 1,
                    MutationSeverity::Medium => medium_count += 1,
                    MutationSeverity::Low => low_count += 1,
                }

                let mut new_mutant = mutant.clone();
                let id_res = store
                    .add_mutant(mutant.clone())
                    .await
                    .expect("failed to add mutant");

                if let Some(id) = id_res {
                    total_new_mutants += 1;
                    new_mutant.id = id;

                    // Show individual mutant if verbose
                    if args.verbose {
                        info!("  Saved mutant: {}", new_mutant.display(target));
                    }
                }
            }

            // Print per-target summary (streaming output)
            if mutant_count == 0 {
                info!("{}: no mutants generated", target.display());
            } else {
                info!(
                    "{}: {} high, {} medium, {} low severity mutants",
                    target.display(),
                    high_count,
                    medium_count,
                    low_count
                );
            }
        } else {
            error!(
                "Failed to generate mutants for {}: {}",
                target.display(),
                mutants_res.err().unwrap()
            );
        }
    }

    info!(
        "Successfully generated {} and saved {} new mutants for {} target(s)",
        total_mutants,
        total_new_mutants,
        targets.len()
    );

    Ok(())
}
