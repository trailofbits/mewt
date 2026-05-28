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
    fn engine(&self) -> &dyn LanguageEngine;

    fn is_language_name(&self, raw: &str) -> bool;

    /// Whether this resolver accepts the global CLI `--dialect` flag.
    ///
    /// Some language families are dialect-aware without using this flag. For example,
    /// JavaScript selects `js`/`jsx`/`ts`/`tsx` from the language label or file extension,
    /// while Move currently uses the global `--dialect` flag for `.move` files.
    fn supports_cli_dialect_flag(&self, raw: &str) -> bool {
        self.is_language_name(raw)
    }

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&ResolutionDefaults>,
    ) -> Result<String, String>;

    fn resolve_for_explicit_dialect(
        &self,
        explicit_dialect: &str,
        defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<String, String>>;

    fn resolve_for_extension(
        &self,
        extension: &str,
        defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<String, String>>;

    fn canonicalize_label(&self, raw: &str) -> Option<String>;

    fn expand_filter_labels(&self, query: &str) -> Option<Vec<String>>;
}
