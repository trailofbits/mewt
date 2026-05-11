pub mod clean;
pub mod init;
pub mod mutate;
pub mod print;
pub mod purge;
pub mod results;
pub mod run;
pub mod status;
pub mod test;

use log::{info, warn};

use crate::languages::r#move::dialect::is_move_language_name;
use crate::types::Target;
use crate::types::config::ResolvedMoveDialect;

pub(crate) fn log_move_dialect_for_targets(
    targets: &[Target],
    resolved_move_dialect: ResolvedMoveDialect,
    context: &str,
) {
    let has_move_targets = targets
        .iter()
        .any(|target| is_move_language_name(&target.language));
    if !has_move_targets {
        return;
    }

    if resolved_move_dialect.defaulted {
        warn!(
            "Move dialect not explicitly set; defaulting to '{}'. Use --dialect or [languages.move].dialect to select sui|iota|aptos|auto explicitly.",
            resolved_move_dialect.dialect.as_str()
        );
    } else {
        info!(
            "Using Move dialect '{}' for {}",
            resolved_move_dialect.dialect.as_str(),
            context
        );
    }
}

// Re-export commands for easier access
pub use clean::execute_clean;
pub use init::execute_init;
pub use mutate::execute_mutate;
pub use print::execute_print;
pub use purge::execute_purge;
pub use results::execute_results;
pub use run::execute_run;
pub use status::execute_status;
pub use test::execute_test;
