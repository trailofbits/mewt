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
pub struct TargetsConfig {
    /// Glob patterns for target inclusion (e.g., "src/**/*.rs")
    pub include: Option<Vec<String>>,
    /// Substrings for path exclusion (e.g., "node_modules")
    pub ignore: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedTargets {
    pub include: Vec<String>,
    pub ignore: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RunConfig {
    /// Whitelist specific mutation types by slug (None = all enabled)
    pub mutations: Option<Vec<String>>,
    pub comprehensive: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    // Top-level fields
    pub db: Option<String>,

    // Nested sections
    pub log: Option<LogConfig>,
    pub test: Option<TestConfig>,
    pub targets: Option<TargetsConfig>,
    pub run: Option<RunConfig>,
}

impl Config {
    pub fn db(&self) -> String {
        self.db
            .clone()
            .unwrap_or_else(|| format!("{}.sqlite", get_namespace()))
    }

    pub fn log(&self) -> LogConfig {
        self.log.clone().unwrap_or_default()
    }

    pub fn test(&self) -> TestConfig {
        self.test.clone().unwrap_or_default()
    }

    pub fn targets(&self) -> Option<&TargetsConfig> {
        self.targets.as_ref()
    }

    pub fn run(&self) -> Option<&RunConfig> {
        self.run.as_ref()
    }

    /// Resolve target configuration with CLI overrides (complete replacement)
    pub fn resolve_targets(
        &self,
        cli_targets: &[String],
        cli_ignore: Option<&str>,
    ) -> std::io::Result<ResolvedTargets> {
        // CLI completely replaces config
        let include = if !cli_targets.is_empty() {
            cli_targets.to_vec()
        } else if let Some(config_include) = self.targets().and_then(|t| t.include.as_ref()) {
            config_include.clone()
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No targets specified. Provide targets via CLI or config [targets].include",
            ));
        };

        let ignore = if let Some(cli_ign) = cli_ignore {
            cli_ign
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            self.targets()
                .and_then(|t| t.ignore.clone())
                .unwrap_or_default()
        };

        Ok(ResolvedTargets { include, ignore })
    }

    /// Resolve mutations with CLI override (complete replacement)
    pub fn resolve_mutations(&self, cli_mutations: Option<&str>) -> Option<Vec<String>> {
        cli_mutations
            .map(|s| {
                s.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .or_else(|| self.run().and_then(|r| r.mutations.clone()))
    }

    /// Resolve test command with CLI override
    pub fn resolve_test_cmd(&self, cli_test_cmd: Option<&str>) -> Option<String> {
        cli_test_cmd
            .map(|s| s.to_string())
            .or_else(|| self.test().cmd().map(|s| s.to_string()))
    }

    /// Resolve test timeout with CLI override
    pub fn resolve_test_timeout(&self, cli_timeout: Option<u32>) -> Option<u32> {
        cli_timeout.or_else(|| self.test().timeout())
    }

    pub fn to_effective(&self) -> Self {
        Self {
            db: Some(self.db().to_string()),
            log: Some(self.log().to_effective()),
            test: Some(self.test().to_effective()),
            targets: self.targets.clone(),
            run: self.run.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub db: Option<String>,
    pub log_level: Option<String>,
    pub log_color: Option<String>, // "on" | "off"
}

static NAMESPACE: OnceCell<String> = OnceCell::new();
static CONFIG_FILENAME: OnceCell<String> = OnceCell::new();
static CONFIG: OnceCell<Config> = OnceCell::new();

pub fn set_namespace(namespace: &str) {
    let _ = NAMESPACE.set(namespace.to_string());
    // Also set config filename based on namespace if not already set
    if CONFIG_FILENAME.get().is_none() {
        let _ = CONFIG_FILENAME.set(format!("{}.toml", namespace));
    }
}

pub fn get_namespace() -> &'static str {
    NAMESPACE.get().map(|s| s.as_str()).unwrap()
}

pub fn set_config_filename(filename: &str) {
    let _ = CONFIG_FILENAME.set(filename.to_string());
}

pub fn get_config_filename() -> &'static str {
    CONFIG_FILENAME.get().map(|s| s.as_str()).unwrap()
}

pub fn config() -> &'static Config {
    CONFIG.get_or_init(|| {
        let mut cfg = Config::default();
        // Apply nearest config file found by walking up from cwd
        if let Some(path) = find_nearest_config_file() {
            if let Some(file_cfg) = read_config_file(&path) {
                apply_file_config(&mut cfg, &file_cfg);
            }
        }
        cfg
    })
}

pub fn init_with_overrides(overrides: &CliOverrides) {
    let mut cfg = Config::default();

    // 1) Config file: walk up from cwd and use the first config file found
    if let Some(path) = find_nearest_config_file() {
        if let Some(file_cfg) = read_config_file(&path) {
            apply_file_config(&mut cfg, &file_cfg);
        }
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

    // Merge targets section
    if let Some(file_targets) = &file.targets {
        cfg.targets = Some(file_targets.clone());
    }

    // Merge run section
    if let Some(file_run) = &file.run {
        cfg.run = Some(file_run.clone());
    }
}

fn apply_cli_overrides(cfg: &mut Config, overrides: &CliOverrides) {
    // Top-level overrides
    if overrides.db.is_some() {
        cfg.db = overrides.db.clone();
    }

    // Log overrides
    let mut log = cfg.log.clone().unwrap_or_default();
    if let Some(level) = &overrides.log_level {
        if !level.trim().is_empty() {
            log.level = Some(level.trim().to_string());
        }
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

pub fn is_slug_enabled(slug: &str, mutations: Option<&[String]>) -> bool {
    if let Some(list) = mutations {
        return list.iter().any(|s| s == slug);
    }
    true
}

pub fn is_path_excluded(path: &Path, ignore_patterns: &[String]) -> bool {
    if ignore_patterns.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    ignore_patterns
        .iter()
        .filter(|p| !p.is_empty())
        .any(|pat| path_str.contains(pat))
}

pub fn resolve_test_for_path(
    path: &Path,
    resolved_cmd: Option<&str>,
    resolved_timeout: Option<u32>,
) -> (Option<String>, Option<u32>) {
    let test = config().test();

    // If we have a resolved command from CLI, use it
    if let Some(cmd) = resolved_cmd {
        if !cmd.trim().is_empty() {
            return (Some(cmd.to_string()), resolved_timeout);
        }
    }

    // Per-target rules: first match wins
    let path_buf = PathBuf::from(path);
    for rule in test.per_target() {
        if glob_matches(&rule.glob, &path_buf) {
            if let Some(cmd) = &rule.cmd {
                let timeout = resolved_timeout.or(rule.timeout).or(test.timeout());
                return (Some(cmd.clone()), timeout);
            }
        }
    }

    // Fallback to global
    (
        test.cmd().map(|s| s.to_string()),
        resolved_timeout.or(test.timeout()),
    )
}

fn glob_matches(pattern: &str, path: &Path) -> bool {
    if let Ok(glob) = globset::Glob::new(pattern) {
        let matcher = glob.compile_matcher();
        return matcher.is_match(path);
    }
    false
}
