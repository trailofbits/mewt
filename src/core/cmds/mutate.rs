use log::{error, info};
use std::sync::Arc;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::MutateArgs;
use crate::types::config::ResolvedTargets;
use crate::types::{AppResult, Target};

pub async fn execute_mutate(
    _args: MutateArgs,
    store: SqlStore,
    registry: Arc<LanguageRegistry>,
    resolved_targets: ResolvedTargets,
    mutations: Option<Vec<String>>,
) -> AppResult<()> {
    info!(
        "Generating mutants for targets: {:?}",
        resolved_targets.include
    );

    let mutations_slice = mutations.as_deref();

    // Load targets from the resolved configuration
    let targets =
        Target::load_targets(&resolved_targets, &store, &registry, mutations_slice).await?;

    let mut total_mutants = 0;

    // Generate and save mutants for each target
    let mut new_mutants = 0;
    for target in targets.iter() {
        let mutants_res = target.generate_mutants(&registry, mutations_slice);
        if let Ok(mutants) = mutants_res {
            info!(
                "Generated {} mutants for {}",
                mutants.len(),
                target.display()
            );
            total_mutants += mutants.len();

            for mutant in mutants {
                let mut new_mutant = mutant.clone();
                let id_res = store
                    .add_mutant(mutant)
                    .await
                    .expect("failed to add mutant");
                if let Some(id) = id_res {
                    new_mutants += 1;
                    new_mutant.id = id;
                    info!("Saved mutant: {}", new_mutant.display(target));
                }
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
        new_mutants,
        targets.len()
    );

    Ok(())
}
