use std::fs;
use std::io::Write;
use std::path::PathBuf;

use log::{info, warn};

use crate::types::AppResult;
use crate::types::config::{get_config_filename, get_namespace};

const EXAMPLE_CONFIG: &str = include_str!("../../example.toml");

pub async fn execute_init() -> AppResult<()> {
    info!("Initializing config file...");

    let config_filename = get_config_filename();
    let cfg_path = PathBuf::from(config_filename);
    if cfg_path.exists() {
        warn!("{} already exists; leaving it unchanged", config_filename);
    } else {
        // Replace {namespace} placeholder with actual namespace
        let namespace = get_namespace();
        let config_content = EXAMPLE_CONFIG.replace("{namespace}", namespace);

        let mut f = fs::File::create(&cfg_path)?;
        f.write_all(config_content.as_bytes())?;
        info!("Created {}", cfg_path.display());
    }

    Ok(())
}
