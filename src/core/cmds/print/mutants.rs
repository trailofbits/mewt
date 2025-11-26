use console::style;
use log::info;

use crate::SqlStore;
use crate::types::{AppResult, Target};

pub async fn execute(store: SqlStore, target_path: Option<String>) -> AppResult<()> {
    // Get targets filtered by path
    let filtered_targets = Target::filter_by_path(&store, target_path.clone()).await?;
    if filtered_targets.is_empty() {
        info!("No targets found");
        return Ok(());
    }

    // Group mutants by target
    for target in filtered_targets {
        info!("{}", style(format!("Target: {}", target.display())).bold());

        // Get all mutants for this target
        let mutants = store.get_mutants(target.id).await?;
        if mutants.is_empty() {
            info!("  No mutants found for this target");
            continue;
        }

        // Print mutants
        for mutant in mutants {
            info!("  {}", mutant.display(&target));
        }

        info!(""); // Empty line between targets
    }

    Ok(())
}
