use std::fs;
use std::path::{Path, PathBuf};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LogFileConfig {
    pub level: Option<String>, // e.g., "info", "warn"
    pub color: Option<bool>,   // true/false; None by omission
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GeneralFileConfig {
    pub db: Option<String>,
    pub ignore_targets: Option<Vec<String>>, // substring patterns
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MutationsFileConfig {
    pub slugs: Option<Vec<String>>, // global whitelist of mutation slugs
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TestFileConfig {
    pub cmd: Option<String>,
    pub timeout: Option<u32>,
    pub per_target: Option<Vec<PerTargetTestFileConfig>>, // ordered, first match wins
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FileConfig {
    pub log: Option<LogFileConfig>,
    pub general: Option<GeneralFileConfig>,
    pub mutations: Option<MutationsFileConfig>,
    pub test: Option<TestFileConfig>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LogConfig {
    pub level: String,       // resolved level; default "info"
    pub color: Option<bool>, // Some(true)=force on, Some(false)=force off, None=auto
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GeneralConfig {
    pub db: String,                  // resolved db path; default "mewt.sqlite"
    pub ignore_targets: Vec<String>, // merged substrings
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MutationsConfig {
    pub slugs: Option<Vec<String>>, // highest-priority non-empty overrides
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TestConfig {
    pub cmd: Option<String>,                // resolved
    pub timeout: Option<u32>,               // seconds
    pub per_target: Vec<PerTargetTestRule>, // ordered, first match wins
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GlobalConfig {
    pub general: GeneralConfig,
    pub mutations: MutationsConfig,
    pub test: TestConfig,
    pub log: LogConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PerTargetTestFileConfig {
    pub glob: String,
    pub cmd: Option<String>,
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PerTargetTestRule {
    pub glob: String,
    pub cmd: String,
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub db: Option<String>,
    pub log_level: Option<String>,
    pub log_color: Option<String>,       // "on" | "off"
    pub ignore_targets: Option<String>,  // csv
    pub mutations_slugs: Option<String>, // csv
    pub test_cmd: Option<String>,
    pub test_timeout: Option<u32>,
}

static CONFIG_FILENAME: OnceCell<String> = OnceCell::new();
static CONFIG: OnceCell<GlobalConfig> = OnceCell::new();

pub fn set_config_filename(filename: &str) {
    let _ = CONFIG_FILENAME.set(filename.to_string());
}

pub fn get_config_filename() -> &'static str {
    CONFIG_FILENAME.get().map(|s| s.as_str()).unwrap_or("mewt.toml")
}

pub fn config() -> &'static GlobalConfig {
    CONFIG.get_or_init(|| {
        let mut cfg = default_global_config();
        // Apply nearest config file found by walking up from cwd
        if let Some(path) = find_nearest_config_file()
            && let Some(file_cfg) = read_config_file(&path)
        {
            apply_file_config(&mut cfg, &file_cfg);
        }
        cfg
    })
}

pub fn init_with_overrides(overrides: &CliOverrides) {
    let mut cfg = default_global_config();

    // 1) Config file: walk up from cwd and use the first config file found
    if let Some(path) = find_nearest_config_file()
        && let Some(file_cfg) = read_config_file(&path)
    {
        apply_file_config(&mut cfg, &file_cfg);
    }

    // 2) CLI arguments (highest priority). Only override if user specified.
    apply_cli_overrides(&mut cfg, overrides);

    let _ = CONFIG.set(cfg);
}

pub fn default_global_config() -> GlobalConfig {
    GlobalConfig {
        general: GeneralConfig {
            db: "mewt.sqlite".to_string(),
            ignore_targets: Vec::new(),
        },
        mutations: MutationsConfig { slugs: None },
        test: TestConfig {
            cmd: None,
            timeout: None,
            per_target: Vec::new(),
        },
        log: LogConfig {
            level: "info".to_string(),
            color: None,
        },
    }
}

fn read_config_file(path: &Path) -> Option<FileConfig> {
    match fs::read_to_string(path) {
        Ok(contents) => toml::from_str::<FileConfig>(&contents).ok(),
        Err(_) => None,
    }
}

fn apply_file_config(cfg: &mut GlobalConfig, file: &FileConfig) {
    if let Some(log) = &file.log {
        if let Some(level) = &log.level {
            cfg.log.level = level.clone();
        }
        if let Some(color) = log.color {
            cfg.log.color = Some(color);
        }
    }
    if let Some(r#gen) = &file.general {
        if let Some(db) = &r#gen.db {
            cfg.general.db = db.clone();
        }
        if let Some(globs) = &r#gen.ignore_targets {
            cfg.general.ignore_targets.extend(globs.clone());
        }
    }
    if let Some(muts) = &file.mutations
        && let Some(slugs) = &muts.slugs
        && !slugs.is_empty()
    {
        cfg.mutations.slugs = Some(slugs.clone()); // override semantics
    }
    if let Some(test) = &file.test {
        if let Some(cmd) = &test.cmd {
            cfg.test.cmd = Some(cmd.clone());
        }
        if let Some(timeout) = test.timeout {
            cfg.test.timeout = Some(timeout);
        }
        if let Some(per) = &test.per_target {
            for rule in per {
                if let Some(cmd) = &rule.cmd
                    && !cmd.trim().is_empty()
                {
                    cfg.test.per_target.push(PerTargetTestRule {
                        glob: rule.glob.clone(),
                        cmd: cmd.clone(),
                        timeout: rule.timeout,
                    });
                }
            }
        }
    }
}


fn apply_cli_overrides(cfg: &mut GlobalConfig, overrides: &CliOverrides) {
    // Global overrides
    if let Some(db) = overrides.db.as_ref() {
        cfg.general.db = db.clone();
    }
    if let Some(level) = overrides.log_level.as_ref()
        && !level.trim().is_empty()
    {
        cfg.log.level = level.trim().to_string();
    }
    if let Some(color) = overrides.log_color.as_ref() {
        match color.to_lowercase().as_str() {
            "on" => cfg.log.color = Some(true),
            "off" => cfg.log.color = Some(false),
            _ => {}
        }
    }
    if let Some(ignore_csv) = overrides.ignore_targets.as_ref() {
        cfg.general.ignore_targets.extend(parse_csv(ignore_csv));
    }

    // Mutations slugs override (highest non-empty wins)
    if let Some(muts_csv) = overrides.mutations_slugs.as_ref() {
        let list = parse_csv(muts_csv);
        if !list.is_empty() {
            cfg.mutations.slugs = Some(list);
        }
    }

    // Test overrides
    if let Some(cmd) = overrides.test_cmd.as_ref()
        && !cmd.trim().is_empty()
    {
        cfg.test.cmd = Some(cmd.clone());
    }
    if let Some(timeout) = overrides.test_timeout {
        cfg.test.timeout = Some(timeout);
    }
}

fn parse_csv(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn find_nearest_config_file() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let config_filename = get_config_filename();
    for dir in cwd.ancestors() {
        let candidate = dir.join(config_filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn colors_enabled() -> bool {
    match config().log.color {
        Some(force) => force,
        None => console::colors_enabled(),
    }
}

pub fn is_slug_enabled(slug: &str) -> bool {
    if let Some(list) = &config().mutations.slugs {
        return list.iter().any(|s| s == slug);
    }
    true
}

pub fn is_path_excluded(path: &Path) -> bool {
    let patterns = &config().general.ignore_targets;
    if patterns.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    patterns
        .iter()
        .filter(|p| !p.is_empty())
        .any(|pat| path_str.contains(pat))
}

pub fn resolve_test_for_path_with_cli(
    path: &Path,
    cli_test_cmd: &Option<String>,
    cli_timeout: Option<u32>,
) -> (Option<String>, Option<u32>) {
    // CLI has highest precedence
    if let Some(cmd) = cli_test_cmd.as_ref()
        && !cmd.trim().is_empty()
    {
        let timeout = cli_timeout.or(config().test.timeout);
        return (Some(cmd.clone()), timeout);
    }

    // Per-target rules: first match wins
    let path_buf = PathBuf::from(path);
    for rule in &config().test.per_target {
        if glob_matches(&rule.glob, &path_buf) {
            let timeout = cli_timeout.or(rule.timeout).or(config().test.timeout);
            return (Some(rule.cmd.clone()), timeout);
        }
    }

    // Fallback to global
    (
        config().test.cmd.clone(),
        cli_timeout.or(config().test.timeout),
    )
}

fn glob_matches(pattern: &str, path: &Path) -> bool {
    if let Ok(glob) = globset::Glob::new(pattern) {
        let matcher = glob.compile_matcher();
        return matcher.is_match(path);
    }
    false
}
