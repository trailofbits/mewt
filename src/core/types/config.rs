use std::collections::BTreeMap;
use std::fs;
use std::hash::Hash as StdHash;
use std::io;
use std::path::{Path, PathBuf};

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::core::resolver::{DialectDefault, ResolutionDefaults};
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
pub struct PerTargetRule {
    pub glob: String,
    pub test: Option<TestConfig>,
    pub run: Option<RunConfig>,
    pub languages: Option<LanguagesConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TestConfig {
    pub cmd: Option<String>,
    pub timeout: Option<u32>,
    /// Deprecated: use top-level [[per_target]] with test.cmd/test.timeout.
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq, StdHash)]
pub struct RunConfig {
    /// Whitelist specific mutation types by slug (None = all enabled)
    pub mutations: Option<Vec<String>>,
    pub comprehensive: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LanguageConfig {
    pub dialect: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LanguagesConfig {
    #[serde(flatten)]
    pub families: BTreeMap<String, LanguageConfig>,
}

impl LanguagesConfig {
    pub fn get(&self, family: &str) -> Option<&LanguageConfig> {
        self.families.get(&family.to_ascii_lowercase())
    }

    pub fn insert(&mut self, family: impl Into<String>, config: LanguageConfig) {
        self.families
            .insert(family.into().to_ascii_lowercase(), config);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &LanguageConfig)> {
        self.families.iter()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    // Top-level fields
    pub db: Option<String>,

    // Nested sections
    pub log: Option<LogConfig>,
    pub targets: Option<TargetsConfig>,

    pub languages: Option<LanguagesConfig>,
    pub test: Option<TestConfig>,
    pub run: Option<RunConfig>,
    pub per_target: Option<Vec<PerTargetRule>>,
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

    pub fn per_target(&self) -> &[PerTargetRule] {
        self.per_target.as_deref().unwrap_or(&[])
    }

    /// Resolve target configuration with CLI overrides (complete replacement)
    pub fn resolve_targets(
        &self,
        cli_include: &[String],
        cli_ignore: Option<&str>,
    ) -> std::io::Result<ResolvedTargets> {
        // CLI completely replaces config
        let include = if !cli_include.is_empty() {
            cli_include.to_vec()
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

    pub fn resolve_run_for_path(
        &self,
        path: &Path,
        cli_mutations: Option<&[String]>,
        cli_comprehensive: bool,
    ) -> (Option<Vec<String>>, bool) {
        let path_buf = PathBuf::from(path);
        let per_target_run = self
            .per_target()
            .iter()
            .find(|rule| glob_matches(&rule.glob, &path_buf) && rule.run.is_some())
            .and_then(|rule| rule.run.as_ref());

        let mutations = cli_mutations
            .map(|m| m.to_vec())
            .or_else(|| per_target_run.and_then(|r| r.mutations.clone()))
            .or_else(|| self.run().and_then(|r| r.mutations.clone()));

        let comprehensive = cli_comprehensive
            || per_target_run
                .and_then(|r| r.comprehensive)
                .unwrap_or(false)
            || self.run().and_then(|r| r.comprehensive).unwrap_or(false);

        (mutations, comprehensive)
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

    pub fn resolve_language_defaults(
        &self,
        cli_dialect_family: Option<&str>,
        cli_dialect: Option<&str>,
    ) -> io::Result<ResolutionDefaults> {
        if cli_dialect.is_some() && cli_dialect_family.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--dialect was provided, but no registered language accepts CLI dialects",
            ));
        }

        let mut defaults = ResolutionDefaults::default();

        if let Some(languages) = &self.languages {
            for (family, language_cfg) in languages.iter() {
                if let Some(dialect) = language_cfg.dialect.as_deref() {
                    defaults.default_dialects.insert(
                        family.to_ascii_lowercase(),
                        DialectDefault {
                            dialect: dialect.to_string(),
                            defaulted: false,
                        },
                    );
                }
            }
        }

        if let (Some(family), Some(dialect)) = (cli_dialect_family, cli_dialect) {
            defaults.default_dialects.insert(
                family.to_ascii_lowercase(),
                DialectDefault {
                    dialect: dialect.to_string(),
                    defaulted: false,
                },
            );
        }

        Ok(defaults)
    }

    pub fn resolve_language_defaults_for_path(
        &self,
        path: &Path,
        base_defaults: &ResolutionDefaults,
        cli_dialect_family: Option<&str>,
    ) -> ResolutionDefaults {
        let mut defaults = base_defaults.clone();
        let cli_family = cli_dialect_family.map(str::to_ascii_lowercase);
        let path_buf = PathBuf::from(path);

        for rule in self.per_target() {
            if !glob_matches(&rule.glob, &path_buf) {
                continue;
            }
            let Some(languages) = &rule.languages else {
                continue;
            };
            for (family, language_cfg) in languages.iter() {
                if cli_family.as_deref() == Some(family.as_str()) {
                    continue;
                }
                if let Some(dialect) = language_cfg.dialect.as_deref() {
                    defaults.default_dialects.insert(
                        family.to_ascii_lowercase(),
                        DialectDefault {
                            dialect: dialect.to_string(),
                            defaulted: false,
                        },
                    );
                }
            }
            break;
        }

        defaults
    }

    pub fn to_effective(&self) -> Self {
        Self {
            db: Some(self.db().to_string()),
            log: Some(self.log().to_effective()),
            targets: self.targets.clone(),
            test: Some(self.test().to_effective()),
            run: self.run.clone(),
            languages: self.languages.clone(),
            per_target: if self.per_target().is_empty() {
                None
            } else {
                Some(self.per_target().to_vec())
            },
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

    // Merge top-level per-target rules
    if let Some(file_per_target) = &file.per_target {
        let mut rules = cfg.per_target().to_vec();
        rules.extend(file_per_target.clone());
        cfg.per_target = Some(rules);
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

        for (family, file_language_cfg) in file_languages.iter() {
            let mut language_cfg = languages.get(family).cloned().unwrap_or_default();
            if file_language_cfg.dialect.is_some() {
                language_cfg.dialect = file_language_cfg.dialect.clone();
            }
            languages.insert(family, language_cfg);
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

    // Top-level per-target rules: first matching test override wins
    let path_buf = PathBuf::from(path);
    for rule in config().per_target() {
        if !glob_matches(&rule.glob, &path_buf) {
            continue;
        }
        let Some(rule_test) = &rule.test else {
            continue;
        };
        let cmd = rule_test.cmd().or_else(|| test.cmd()).map(str::to_string);
        let timeout = resolved_timeout.or(rule_test.timeout()).or(test.timeout());
        return (cmd, timeout);
    }

    // Deprecated nested per-target rules: first match wins
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

    fn config_with_language_dialect(family: &str, dialect: &str) -> Config {
        let mut languages = LanguagesConfig::default();
        languages.insert(
            family,
            LanguageConfig {
                dialect: Some(dialect.to_string()),
            },
        );
        Config {
            languages: Some(languages),
            ..Config::default()
        }
    }

    #[test]
    fn resolve_language_defaults_prefers_cli_over_config_for_cli_family() {
        let cfg = config_with_language_dialect("example", "configured");
        let resolved = cfg
            .resolve_language_defaults(Some("example"), Some("cli"))
            .expect("valid dialect defaults");

        let language_default = resolved.default_dialects.get("example").unwrap();
        assert_eq!(language_default.dialect, "cli");
        assert!(!language_default.defaulted);
    }

    #[test]
    fn resolve_language_defaults_uses_generic_language_config() {
        let cfg = config_with_language_dialect("example", "configured");
        let resolved = cfg
            .resolve_language_defaults(None, None)
            .expect("valid dialect defaults");

        let language_default = resolved.default_dialects.get("example").unwrap();
        assert_eq!(language_default.dialect, "configured");
        assert!(!language_default.defaulted);
    }

    #[test]
    fn resolve_language_defaults_does_not_insert_language_specific_defaults() {
        let cfg = Config::default();
        let resolved = cfg
            .resolve_language_defaults(None, None)
            .expect("valid dialect defaults");

        assert!(resolved.default_dialects.is_empty());
    }

    #[test]
    fn resolve_language_defaults_rejects_cli_dialect_without_accepting_family() {
        let cfg = Config::default();
        assert!(
            cfg.resolve_language_defaults(None, Some("dialect"))
                .is_err()
        );
    }

    #[test]
    fn parses_top_level_per_target_dot_notation() {
        let cfg: Config = toml::from_str(
            r#"
                [[per_target]]
                glob = "src/auth/login.rs"
                test.cmd = "cargo test auth_login"
                test.timeout = 60
                run.mutations = ["ER", "CR"]
                run.comprehensive = true
                languages.move.dialect = "aptos"
            "#,
        )
        .expect("valid per-target config");

        let rule = cfg.per_target().first().expect("per-target rule");
        assert_eq!(rule.glob, "src/auth/login.rs");
        assert_eq!(
            rule.test.as_ref().and_then(|t| t.cmd.as_deref()),
            Some("cargo test auth_login")
        );
        assert_eq!(rule.test.as_ref().and_then(|t| t.timeout), Some(60));
        assert_eq!(
            rule.run.as_ref().and_then(|r| r.mutations.clone()),
            Some(vec!["ER".to_string(), "CR".to_string()])
        );
        assert_eq!(rule.run.as_ref().and_then(|r| r.comprehensive), Some(true));
        assert_eq!(
            rule.languages
                .as_ref()
                .and_then(|l| l.get("move"))
                .and_then(|l| l.dialect.as_deref()),
            Some("aptos")
        );
    }

    #[test]
    fn resolve_run_for_path_prefers_cli_then_per_target_then_global() {
        let cfg: Config = toml::from_str(
            r#"
                [run]
                mutations = ["GLOBAL"]
                comprehensive = false

                [[per_target]]
                glob = "src/special.rs"
                run.mutations = ["LOCAL"]
                run.comprehensive = true
            "#,
        )
        .expect("valid config");

        let (mutations, comprehensive) =
            cfg.resolve_run_for_path(Path::new("src/special.rs"), None, false);
        assert_eq!(mutations, Some(vec!["LOCAL".to_string()]));
        assert!(comprehensive);

        let cli = vec!["CLI".to_string()];
        let (mutations, comprehensive) =
            cfg.resolve_run_for_path(Path::new("src/special.rs"), Some(&cli), false);
        assert_eq!(mutations, Some(vec!["CLI".to_string()]));
        assert!(comprehensive);

        let (mutations, comprehensive) =
            cfg.resolve_run_for_path(Path::new("src/other.rs"), None, false);
        assert_eq!(mutations, Some(vec!["GLOBAL".to_string()]));
        assert!(!comprehensive);
    }

    #[test]
    fn per_target_language_defaults_override_global_but_not_cli() {
        let cfg: Config = toml::from_str(
            r#"
                [languages.move]
                dialect = "sui"

                [[per_target]]
                glob = "sources/aptos/**/*.move"
                languages.move.dialect = "aptos"
            "#,
        )
        .expect("valid config");
        let base = cfg.resolve_language_defaults(None, None).unwrap();

        let defaults = cfg.resolve_language_defaults_for_path(
            Path::new("sources/aptos/module.move"),
            &base,
            None,
        );
        assert_eq!(
            defaults
                .default_dialects
                .get("move")
                .map(|d| d.dialect.as_str()),
            Some("aptos")
        );

        let cli_base = cfg
            .resolve_language_defaults(Some("move"), Some("iota"))
            .unwrap();
        let defaults = cfg.resolve_language_defaults_for_path(
            Path::new("sources/aptos/module.move"),
            &cli_base,
            Some("move"),
        );
        assert_eq!(
            defaults
                .default_dialects
                .get("move")
                .map(|d| d.dialect.as_str()),
            Some("iota")
        );
    }
}
