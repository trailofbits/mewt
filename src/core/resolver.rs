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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialectPolicy {
    pub dialects: &'static [&'static str],
}

impl DialectPolicy {
    pub const NONE: Self = Self { dialects: &[] };

    pub fn has_dialects(&self) -> bool {
        !self.dialects.is_empty()
    }

    pub fn contains(&self, raw: &str) -> bool {
        self.dialects
            .iter()
            .any(|dialect| dialect.eq_ignore_ascii_case(raw.trim()))
    }

    pub fn expected(&self) -> String {
        self.dialects.join(", ")
    }
}

pub struct ResolutionRequest<'a> {
    pub path: &'a Path,
    pub explicit_language: Option<&'a str>,
    pub defaults: Option<&'a ResolutionDefaults>,
}

pub trait LanguageResolver: Send + Sync {
    /// Stable family key used for generic config lookup and diagnostics.
    fn family(&self) -> &'static str;

    /// All concrete engines owned by this family resolver.
    ///
    /// Dialect-aware families return one engine per concrete dialect label.
    fn engines(&self) -> Vec<&dyn LanguageEngine>;

    /// Dialect keys exposed by this family, if any.
    ///
    /// These keys may be selected by dialect-qualified labels such as
    /// `family/dialect` and by project/per-target config dialect selections.
    fn dialect_policy(&self) -> DialectPolicy {
        DialectPolicy::NONE
    }

    /// Resolve a request to one concrete engine.
    ///
    /// This is the dialect-resolution boundary. Implementations may inspect the
    /// path, explicit language label, and language defaults,
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
