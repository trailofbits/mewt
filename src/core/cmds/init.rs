use std::fs;
use std::io::Write;
use std::path::PathBuf;

use log::{info, warn};

use crate::types::AppResult;
use crate::types::config::get_config_filename;

const EXAMPLE_CONFIG: &str = include_str!("../../example.toml");

pub async fn execute_init() -> AppResult<()> {
    info!("Initializing workspace...");

    let config_filename = get_config_filename();
    let cfg_path = PathBuf::from(config_filename);
    if cfg_path.exists() {
        warn!("{} already exists; leaving it unchanged", config_filename);
    } else {
        let mut f = fs::File::create(&cfg_path)?;
        f.write_all(EXAMPLE_CONFIG.as_bytes())?;
        info!("Created {}", cfg_path.display());
    }

    Ok(())
}
