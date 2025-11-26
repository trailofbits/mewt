use log::{error, info};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::MutateArgs;
use crate::types::{AppResult, Target};

pub async fn execute_mutate(
    args: MutateArgs,
    store: SqlStore,
    registry: Arc<LanguageRegistry>,
) -> AppResult<()> {
    let target = PathBuf::from(&args.target)
        .canonicalize()
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, format!("Invalid target: {e}")))?;

    info!("Generating mutants for target: {}", target.display());

    // Load targets from the given path
    let targets = Target::load_targets(target, &store, &registry).await?;

    let mut total_mutants = 0;

    // Generate and save mutants for each target
    let mut new_mutants = 0;
    for target in targets.iter() {
        let mutants_res = target.generate_mutants(&registry);
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
