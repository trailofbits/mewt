use std::sync::Arc;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::{AppError, AppResult};

pub mod mutant;
pub mod mutants;
pub mod mutations;
pub mod outcomes;
pub mod targets;

pub struct ResultsFilters {
    pub target: Option<String>,
    pub verbose: bool,
    pub id: Option<i64>,
    pub all: bool,
    pub status: Option<String>,
    pub language: Option<String>,
    pub mutation_type: Option<String>,
    pub line: Option<u32>,
    pub file: Option<String>,
    pub format: String,
}

pub struct MutantsFilters {
    pub target: Option<String>,
    pub line: Option<u32>,
    pub file: Option<String>,
    pub mutation_type: Option<String>,
    pub tested: bool,
    pub untested: bool,
    pub format: String,
}

pub enum PrintCommand {
    Mutations(Option<String>),
    Results(ResultsFilters),
    Targets,
    Mutant(i64),
    Mutants(MutantsFilters),
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
        PrintCommand::Mutations(language) => mutations::execute(language, &registry)
            .await
            .map_err(AppError::Custom),
        PrintCommand::Results(filters) => {
            if let Some(store) = store {
                outcomes::execute(store, filters, &registry).await
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
