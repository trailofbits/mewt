use std::fs;
use std::path::{Path, PathBuf};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LogConfig {
    pub level: Option<String>,
    pub color: Option<bool>, // None = auto-detect (semantic)
}

impl LogConfig {
    pub fn level(&self) -> &str {
        self.level.as_deref().unwrap_or("info")
    }

    pub fn color(&self) -> Option<bool> {
        self.color // None has semantic meaning (auto-detect)
    }

    pub fn to_effective(&self) -> Self {
        Self {
            level: Some(self.level().to_string()),
            color: self.color,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PerTargetTestRule {
    pub glob: String,
    pub cmd: Option<String>,
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TestConfig {
    pub cmd: Option<String>,
    pub timeout: Option<u32>,
    pub per_target: Option<Vec<PerTargetTestRule>>, // ordered, first match wins
}

impl TestConfig {
    pub fn cmd(&self) -> Option<&str> {
        self.cmd.as_deref()
    }

    pub fn timeout(&self) -> Option<u32> {
        self.timeout
    }

    pub fn per_target(&self) -> &[PerTargetTestRule] {
        self.per_target.as_deref().unwrap_or(&[])
    }

    pub fn to_effective(&self) -> Self {
        Self {
            cmd: self.cmd.clone(),
            timeout: self.timeout,
            per_target: if self.per_target().is_empty() {
                None
            } else {
                Some(self.per_target().to_vec())
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    // Top-level fields
    pub db: Option<String>,
    pub ignore_targets: Option<Vec<String>>,
    pub mutations: Option<Vec<String>>, // None = all enabled (semantic)

    // Nested sections
    pub log: Option<LogConfig>,
    pub test: Option<TestConfig>,
}

impl Config {
    pub fn db(&self) -> &str {
        self.db.as_deref().unwrap_or("mewt.sqlite")
    }

    pub fn ignore_targets(&self) -> &[String] {
        self.ignore_targets.as_deref().unwrap_or(&[])
    }

    pub fn mutations(&self) -> Option<&[String]> {
        self.mutations.as_deref() // None = all enabled (semantic)
    }

    pub fn log(&self) -> LogConfig {
        self.log.clone().unwrap_or_default()
    }

    pub fn test(&self) -> TestConfig {
        self.test.clone().unwrap_or_default()
    }

    pub fn to_effective(&self) -> Self {
        Self {
            db: Some(self.db().to_string()),
            ignore_targets: Some(self.ignore_targets().to_vec()),
            mutations: self.mutations.as_ref().map(|v| v.to_vec()),
            log: Some(self.log().to_effective()),
            test: Some(self.test().to_effective()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub db: Option<String>,
    pub log_level: Option<String>,
    pub log_color: Option<String>,      // "on" | "off"
    pub ignore_targets: Option<String>, // csv
    pub mutations: Option<String>,      // csv
    pub test_cmd: Option<String>,
    pub test_timeout: Option<u32>,
}

static CONFIG_FILENAME: OnceCell<String> = OnceCell::new();
static CONFIG: OnceCell<Config> = OnceCell::new();

pub fn set_config_filename(filename: &str) {
    let _ = CONFIG_FILENAME.set(filename.to_string());
}

pub fn get_config_filename() -> &'static str {
    CONFIG_FILENAME
        .get()
        .map(|s| s.as_str())
        .unwrap_or("mewt.toml")
}

pub fn config() -> &'static Config {
    CONFIG.get_or_init(|| {
        let mut cfg = Config::default();
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
    let mut cfg = Config::default();

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

fn read_config_file(path: &Path) -> Option<Config> {
    match fs::read_to_string(path) {
        Ok(contents) => toml::from_str::<Config>(&contents).ok(),
        Err(_) => None,
    }
}

fn apply_file_config(cfg: &mut Config, file: &Config) {
    // Merge top-level fields
    if file.db.is_some() {
        cfg.db = file.db.clone();
    }
    if let Some(targets) = &file.ignore_targets {
        cfg.ignore_targets = Some(
            cfg.ignore_targets()
                .iter()
                .chain(targets.iter())
                .cloned()
                .collect(),
        );
    }
    if file.mutations.is_some() {
        cfg.mutations = file.mutations.clone(); // override semantics
    }

    // Merge log section
    if let Some(file_log) = &file.log {
        let mut log = cfg.log.clone().unwrap_or_default();
        if file_log.level.is_some() {
            log.level = file_log.level.clone();
        }
        if file_log.color.is_some() {
            log.color = file_log.color;
        }
        cfg.log = Some(log);
    }

    // Merge test section
    if let Some(file_test) = &file.test {
        let mut test = cfg.test.clone().unwrap_or_default();
        if file_test.cmd.is_some() {
            test.cmd = file_test.cmd.clone();
        }
        if file_test.timeout.is_some() {
            test.timeout = file_test.timeout;
        }
        if let Some(file_per_target) = &file_test.per_target {
            let mut rules = test.per_target().to_vec();
            for rule in file_per_target {
                if rule.cmd.as_ref().is_some_and(|c| !c.trim().is_empty()) {
                    rules.push(rule.clone());
                }
            }
            test.per_target = Some(rules);
        }
        cfg.test = Some(test);
    }
}

fn apply_cli_overrides(cfg: &mut Config, overrides: &CliOverrides) {
    // Top-level overrides
    if overrides.db.is_some() {
        cfg.db = overrides.db.clone();
    }
    if let Some(ignore_csv) = &overrides.ignore_targets {
        let existing = cfg.ignore_targets().to_vec();
        let new_targets = parse_csv(ignore_csv);
        cfg.ignore_targets = Some(existing.into_iter().chain(new_targets).collect());
    }
    if let Some(muts_csv) = &overrides.mutations {
        let list = parse_csv(muts_csv);
        if !list.is_empty() {
            cfg.mutations = Some(list);
        }
    }

    // Log overrides
    let mut log = cfg.log.clone().unwrap_or_default();
    if let Some(level) = &overrides.log_level
        && !level.trim().is_empty()
    {
        log.level = Some(level.trim().to_string());
    }
    if let Some(color_str) = &overrides.log_color {
        match color_str.to_lowercase().as_str() {
            "on" => log.color = Some(true),
            "off" => log.color = Some(false),
            _ => {}
        }
    }
    if overrides.log_level.is_some() || overrides.log_color.is_some() {
        cfg.log = Some(log);
    }

    // Test overrides
    let mut test = cfg.test.clone().unwrap_or_default();
    if let Some(cmd) = &overrides.test_cmd
        && !cmd.trim().is_empty()
    {
        test.cmd = Some(cmd.clone());
    }
    if overrides.test_timeout.is_some() {
        test.timeout = overrides.test_timeout;
    }
    if overrides.test_cmd.is_some() || overrides.test_timeout.is_some() {
        cfg.test = Some(test);
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
    match config().log().color() {
        Some(force) => force,
        None => console::colors_enabled(),
    }
}

pub fn is_slug_enabled(slug: &str) -> bool {
    if let Some(list) = config().mutations() {
        return list.iter().any(|s| s == slug);
    }
    true
}

pub fn is_path_excluded(path: &Path) -> bool {
    let patterns = config().ignore_targets();
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
    let test = config().test();

    // CLI has highest precedence
    if let Some(cmd) = cli_test_cmd.as_ref()
        && !cmd.trim().is_empty()
    {
        let timeout = cli_timeout.or(test.timeout());
        return (Some(cmd.clone()), timeout);
    }

    // Per-target rules: first match wins
    let path_buf = PathBuf::from(path);
    for rule in test.per_target() {
        if glob_matches(&rule.glob, &path_buf)
            && let Some(cmd) = &rule.cmd
        {
            let timeout = cli_timeout.or(rule.timeout).or(test.timeout());
            return (Some(cmd.clone()), timeout);
        }
    }

    // Fallback to global
    (
        test.cmd().map(|s| s.to_string()),
        cli_timeout.or(test.timeout()),
    )
}

fn glob_matches(pattern: &str, path: &Path) -> bool {
    if let Ok(glob) = globset::Glob::new(pattern) {
        let matcher = glob.compile_matcher();
        return matcher.is_match(path);
    }
    false
}
