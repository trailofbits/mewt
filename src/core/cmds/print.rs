use std::sync::Arc;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::{AppError, AppResult};

pub mod mutant;
pub mod mutants;
pub mod mutations;
pub mod outcomes;
pub mod targets;

pub enum PrintCommand {
    Mutations(Option<String>),
    Results(Option<String>, bool, Option<i64>, bool), // (target_path, verbose, mutant_id, all)
    Targets,
    Mutant(i64),
    Mutants(Option<String>),
}

pub async fn execute_print(
    command: PrintCommand,
    store: Option<SqlStore>,
    registry: Arc<LanguageRegistry>,
) -> AppResult<()> {
    match command {
        PrintCommand::Mutant(mutant_id) => {
            if let Some(store) = store {
                mutant::execute(store, mutant_id).await
            } else {
                Err(AppError::Custom(
                    "Store is required for printing a mutant".to_string(),
                ))
            }
        }
        PrintCommand::Mutants(target_path) => {
            if let Some(store) = store {
                mutants::execute(store, target_path).await
            } else {
                Err(AppError::Custom(
                    "Store is required for listing mutants".to_string(),
                ))
            }
        }
        PrintCommand::Mutations(language) => mutations::execute(language, &registry)
            .await
            .map_err(AppError::Custom),
        PrintCommand::Results(target_path, verbose, mutant_id, all) => {
            if let Some(store) = store {
                outcomes::execute(store, target_path, verbose, mutant_id, all, &registry).await
            } else {
                Err(AppError::Custom(
                    "Store is required for listing outcomes".to_string(),
                ))
            }
        }
        PrintCommand::Targets => {
            if let Some(store) = store {
                targets::execute(store).await
            } else {
                Err(AppError::Custom(
                    "Store is required for listing targets".to_string(),
                ))
            }
        }
    }
}
