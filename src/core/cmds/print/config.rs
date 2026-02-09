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
        info!("Targets:");
        if let Some(targets) = &effective_config.targets {
            if let Some(include) = &targets.include {
                if include.is_empty() {
                    info!("  include: []");
                } else {
                    info!("  include: [{}]", include.join(", "));
                }
            } else {
                info!("  include: (not set)");
            }

            if let Some(ignore) = &targets.ignore {
                if ignore.is_empty() {
                    info!("  ignore: []");
                } else {
                    info!("  ignore: [{}]", ignore.join(", "));
                }
            } else {
                info!("  ignore: (not set)");
            }
        } else {
            info!("  (not configured)");
        }

        info!("");
        info!("Run:");
        if let Some(run) = &effective_config.run {
            if let Some(mutations) = &run.mutations {
                info!("  mutations: [{}]", mutations.join(", "));
            } else {
                info!("  mutations: all enabled");
            }
            if let Some(comprehensive) = run.comprehensive {
                info!("  comprehensive: {}", comprehensive);
            }
        } else {
            info!("  mutations: all enabled");
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
