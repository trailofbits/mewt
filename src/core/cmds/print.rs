use std::sync::Arc;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::{AppError, AppResult};

pub mod config;
pub mod mutant;
pub mod mutants;
pub mod mutations;
pub mod targets;

pub struct MutantsFilters {
    pub target: Option<String>,
    pub line: Option<u32>,
    pub mutation_type: Option<String>,
    pub tested: bool,
    pub untested: bool,
    pub format: String,
}

pub struct MutationsFilters {
    pub language: Option<String>,
    pub format: String,
}

pub enum PrintCommand {
    Mutations(MutationsFilters),
    Targets(String),
    Mutant(i64),
    Mutants(MutantsFilters),
    Config(String),
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
        PrintCommand::Mutants(filters) => {
            if let Some(store) = store {
                mutants::execute(store, filters).await
            } else {
                Err(AppError::Custom(
                    "Store is required for listing mutants".to_string(),
                ))
            }
        }
        PrintCommand::Mutations(filters) => mutations::execute(filters, &registry)
            .await
            .map_err(AppError::Custom),
        PrintCommand::Targets(format) => {
            if let Some(store) = store {
                targets::execute(store, format).await
            } else {
                Err(AppError::Custom(
                    "Store is required for listing targets".to_string(),
                ))
            }
        }
        PrintCommand::Config(format) => config::execute(format).await,
    }
}
