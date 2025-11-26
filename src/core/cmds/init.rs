use std::fs;
use std::io::Write;
use std::path::PathBuf;

use log::{info, warn};

use crate::types::AppResult;
use crate::types::config::default_global_config;

pub async fn execute_init() -> AppResult<()> {
    info!("Initializing workspace...");

    let cfg_path = PathBuf::from("mewt.toml");
    if cfg_path.exists() {
        warn!("mewt.toml already exists; leaving it unchanged");
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
