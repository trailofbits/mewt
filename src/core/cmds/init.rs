use std::fs;
use std::io::Write;
use std::path::PathBuf;

use log::{info, warn};

use crate::types::AppResult;
use crate::types::config::{default_global_config, get_config_filename};

pub async fn execute_init() -> AppResult<()> {
    info!("Initializing workspace...");

    let config_filename = get_config_filename();
    let cfg_path = PathBuf::from(config_filename);
    if cfg_path.exists() {
        warn!("{} already exists; leaving it unchanged", config_filename);
    } else {
        let defaults = default_global_config();
        let toml = toml::to_string_pretty(&defaults)
            .map_err(|e| crate::types::AppError::Custom(e.to_string()))?;
        let mut f = fs::File::create(&cfg_path)?;
        f.write_all(toml.as_bytes())?;
        info!("Created {}", cfg_path.display());
    }

    Ok(())
}
