use std::collections::HashMap;
use std::path::Path;

use crate::LanguageEngine;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolutionDefaults {
    pub default_language: Option<String>,
    pub default_dialects: HashMap<String, DialectDefault>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialectDefault {
    pub dialect: String,
    pub defaulted: bool,
}

pub struct ResolutionRequest<'a> {
    pub path: &'a Path,
    pub explicit_language: Option<&'a str>,
    pub explicit_dialect: Option<&'a str>,
    pub defaults: Option<&'a ResolutionDefaults>,
}

pub trait LanguageResolver: Send + Sync {
    /// Stable family key used for generic config lookup and diagnostics.
    fn family(&self) -> &'static str;

    /// All concrete engines owned by this family resolver.
    ///
    /// Dialect-aware families return one engine per concrete dialect label.
    fn engines(&self) -> Vec<&dyn LanguageEngine>;

    /// Whether this resolver accepts the global CLI `--dialect` flag.
    fn accepts_cli_dialect(&self) -> bool {
        false
    }

    /// Resolve a request to one concrete engine.
    ///
    /// This is the dialect-resolution boundary. Implementations may inspect the
    /// path, explicit language label, explicit dialect, and language defaults,
    /// but the returned engine must already contain the selected dialect config.
    fn resolve<'a>(
        &'a self,
        request: &ResolutionRequest<'_>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>>;

    /// Expand/canonicalize labels used in filtering contexts.
    ///
    /// Filtering is separate from target resolution because family selectors such
    /// as `move` or `javascript` may expand to many concrete labels.
    fn filter_labels(&self, query: &str) -> Option<Vec<String>>;
}
