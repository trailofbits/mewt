use log::info;

use crate::types::AppResult;
use crate::types::config::config;

pub async fn execute(format: String) -> AppResult<()> {
    let effective_config = config().to_effective();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&effective_config)?);
    } else {
        // Table format
        info!("Effective Configuration:");
        info!("");
        info!("Global:");
        info!("  db: {}", effective_config.db.as_ref().unwrap());

        if let Some(ignore_targets) = &effective_config.ignore_targets {
            if ignore_targets.is_empty() {
                info!("  ignore_targets: []");
            } else {
                info!("  ignore_targets: [{}]", ignore_targets.join(", "));
            }
        }

        if let Some(mutations) = &effective_config.mutations {
            info!("  mutations: [{}]", mutations.join(", "));
        } else {
            info!("  mutations: all enabled");
        }

        info!("");
        info!("Log:");
        if let Some(log) = &effective_config.log {
            info!("  level: {}", log.level.as_ref().unwrap());
            match log.color {
                Some(true) => info!("  color: on"),
                Some(false) => info!("  color: off"),
                None => info!("  color: auto"),
            }
        }

        info!("");
        info!("Test:");
        if let Some(test) = &effective_config.test {
            if let Some(cmd) = &test.cmd {
                info!("  cmd: {}", cmd);
            } else {
                info!("  cmd: (not set)");
            }

            if let Some(timeout) = test.timeout {
                info!("  timeout: {}s", timeout);
            } else {
                info!("  timeout: (not set)");
            }

            if let Some(per_target) = &test.per_target
                && !per_target.is_empty()
            {
                info!("  per_target:");
                for rule in per_target {
                    info!("    - glob: {}", rule.glob);
                    if let Some(cmd) = &rule.cmd {
                        info!("      cmd: {}", cmd);
                    }
                    if let Some(timeout) = rule.timeout {
                        info!("      timeout: {}s", timeout);
                    }
                }
            }
        }
    }

    Ok(())
}
