use log::info;

use crate::SqlStore;
use crate::types::AppResult;

pub async fn execute(store: SqlStore, mutant_id: i64) -> AppResult<()> {
    info!("Getting mutant with id: {mutant_id}");
    let mutant = store.get_mutant(mutant_id).await?;
    let target = store.get_target(mutant.target_id).await?;
    let mutated_target = target.mutate(&mutant)?;
    info!("{mutated_target}");
    Ok(())
}
