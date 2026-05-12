use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::core::registry::{DialectDefault, ResolutionDefaults};
use crate::core::utils::parse_csv;

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

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MoveDialectSetting {
    Sui,
    Iota,
    Aptos,
}

impl MoveDialectSetting {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sui => "sui",
            Self::Iota => "iota",
            Self::Aptos => "aptos",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDialect {
    Sui,
    Iota,
    Aptos,
}

impl MoveDialect {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sui => "sui",
            Self::Iota => "iota",
            Self::Aptos => "aptos",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDialectSource {
    Cli,
    Config,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMoveDialect {
    pub dialect: MoveDialect,
    pub source: MoveDialectSource,
    pub defaulted: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MoveLanguageConfig {
    pub dialect: Option<MoveDialectSetting>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LanguagesConfig {
    #[serde(rename = "move")]
    pub move_language: Option<MoveLanguageConfig>,
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
    pub languages: Option<LanguagesConfig>,
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
            parse_csv::<String>(Some(cli_ign)).unwrap_or_default()
        } else {
            self.targets()
                .and_then(|t| t.ignore.clone())
                .unwrap_or_default()
        };

        Ok(ResolvedTargets { include, ignore })
    }

    /// Resolve mutations with CLI override (complete replacement)
    pub fn resolve_mutations(&self, cli_mutations: Option<&str>) -> Option<Vec<String>> {
        parse_csv::<String>(cli_mutations).or_else(|| self.run().and_then(|r| r.mutations.clone()))
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

    pub fn resolve_move_dialect(
        &self,
        cli_dialect: Option<&str>,
    ) -> io::Result<ResolvedMoveDialect> {
        if let Some(cli_value) = cli_dialect {
            let setting = parse_move_dialect(cli_value)?;
            return Ok(resolve_move_dialect_setting(
                setting,
                MoveDialectSource::Cli,
            ));
        }

        if let Some(setting) = self
            .languages
            .as_ref()
            .and_then(|languages| languages.move_language.as_ref())
            .and_then(|move_cfg| move_cfg.dialect)
        {
            return Ok(resolve_move_dialect_setting(
                setting,
                MoveDialectSource::Config,
            ));
        }

        Ok(ResolvedMoveDialect {
            dialect: MoveDialect::Sui,
            source: MoveDialectSource::Default,
            defaulted: true,
        })
    }

    pub fn resolve_language_defaults(
        &self,
        cli_move_dialect: Option<&str>,
    ) -> io::Result<ResolutionDefaults> {
        let mut defaults = ResolutionDefaults::default();
        let resolved = self.resolve_move_dialect(cli_move_dialect)?;
        defaults.default_dialects.insert(
            "move".to_string(),
            DialectDefault {
                dialect: resolved.dialect.as_str().to_string(),
                defaulted: resolved.defaulted,
            },
        );
        Ok(defaults)
    }

    pub fn to_effective(&self) -> Self {
        Self {
            db: Some(self.db().to_string()),
            log: Some(self.log().to_effective()),
            test: Some(self.test().to_effective()),
            targets: self.targets.clone(),
            run: self.run.clone(),
            languages: self.languages.clone(),
        }
    }
}

fn parse_move_dialect(raw: &str) -> io::Result<MoveDialectSetting> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "sui" => Ok(MoveDialectSetting::Sui),
        "iota" => Ok(MoveDialectSetting::Iota),
        "aptos" => Ok(MoveDialectSetting::Aptos),
        value => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid Move dialect '{value}'. Expected one of: sui, iota, aptos"),
        )),
    }
}

fn resolve_move_dialect_setting(
    setting: MoveDialectSetting,
    source: MoveDialectSource,
) -> ResolvedMoveDialect {
    match setting {
        MoveDialectSetting::Sui => ResolvedMoveDialect {
            dialect: MoveDialect::Sui,
            source,
            defaulted: false,
        },
        MoveDialectSetting::Iota => ResolvedMoveDialect {
            dialect: MoveDialect::Iota,
            source,
            defaulted: false,
        },
        MoveDialectSetting::Aptos => ResolvedMoveDialect {
            dialect: MoveDialect::Aptos,
            source,
            defaulted: false,
        },
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
static CONFIG_PATH: OnceCell<Option<PathBuf>> = OnceCell::new();
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

pub fn set_config_path(path: Option<PathBuf>) {
    let _ = CONFIG_PATH.set(path);
}

pub fn get_config_path() -> Option<&'static PathBuf> {
    CONFIG_PATH.get().and_then(|opt| opt.as_ref())
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

    // 1) Config file: use path set by main (already discovered/validated)
    if let Some(path) = get_config_path() {
        if let Some(file_cfg) = read_config_file(path) {
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

    // Merge languages section
    if let Some(file_languages) = &file.languages {
        let mut languages = cfg.languages.clone().unwrap_or_default();

        if let Some(file_move_cfg) = &file_languages.move_language {
            let mut move_cfg = languages.move_language.unwrap_or_default();
            if file_move_cfg.dialect.is_some() {
                move_cfg.dialect = file_move_cfg.dialect;
            }
            languages.move_language = Some(move_cfg);
        }

        cfg.languages = Some(languages);
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

pub fn find_nearest_config_file() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    // Return None if config filename hasn't been set yet (e.g., in tests)
    let config_filename = CONFIG_FILENAME.get()?.as_str();
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

/// Check if a path is included by current config patterns
/// Returns false if no config is set (requires explicit configuration)
pub fn path_is_included(path: &Path) -> bool {
    let targets_cfg = config().targets();

    // If no config, path is not included (requires explicit configuration)
    let Some(cfg) = targets_cfg else {
        return false;
    };

    let Some(include_patterns) = &cfg.include else {
        return false;
    };

    // Build globset for include patterns
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in include_patterns {
        if let Ok(glob) = globset::Glob::new(pattern) {
            builder.add(glob);
        }
    }

    let Some(include_matcher) = builder.build().ok() else {
        return false;
    };

    // Check if path matches include patterns and is not ignored
    let ignore_patterns = cfg.ignore.as_deref().unwrap_or(&[]);
    let is_included = include_matcher.is_match(path);
    let is_ignored = is_path_excluded(path, ignore_patterns);

    is_included && !is_ignored
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

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_move_dialect(dialect: MoveDialectSetting) -> Config {
        Config {
            languages: Some(LanguagesConfig {
                move_language: Some(MoveLanguageConfig {
                    dialect: Some(dialect),
                }),
            }),
            ..Config::default()
        }
    }

    #[test]
    fn resolve_move_dialect_prefers_cli_over_config() {
        let cfg = config_with_move_dialect(MoveDialectSetting::Iota);
        let resolved = cfg
            .resolve_move_dialect(Some("sui"))
            .expect("valid dialect");

        assert_eq!(resolved.dialect, MoveDialect::Sui);
        assert_eq!(resolved.source, MoveDialectSource::Cli);
        assert!(!resolved.defaulted);
    }

    #[test]
    fn resolve_move_dialect_uses_config_when_cli_missing() {
        let cfg = config_with_move_dialect(MoveDialectSetting::Iota);
        let resolved = cfg.resolve_move_dialect(None).expect("valid dialect");

        assert_eq!(resolved.dialect, MoveDialect::Iota);
        assert_eq!(resolved.source, MoveDialectSource::Config);
        assert!(!resolved.defaulted);
    }

    #[test]
    fn resolve_move_dialect_defaults_to_sui_when_missing() {
        let cfg = Config::default();
        let resolved = cfg.resolve_move_dialect(None).expect("default dialect");

        assert_eq!(resolved.dialect, MoveDialect::Sui);
        assert_eq!(resolved.source, MoveDialectSource::Default);
        assert!(resolved.defaulted);
    }

    #[test]
    fn resolve_move_dialect_rejects_invalid_cli_value() {
        let cfg = Config::default();
        assert!(cfg.resolve_move_dialect(Some("wat")).is_err());
    }
}
