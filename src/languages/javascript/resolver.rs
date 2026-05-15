use crate::core::registry::{
    LanguageResolver, ResolutionDefaults, ResolutionSource, ResolvedLanguageSelection,
};
use crate::languages::javascript::dialect::{
    JavaScriptDialect, dialect_from_language_name, is_javascript_language_name,
    language_name_for_dialect,
};

pub struct JavaScriptLanguageResolver;

impl JavaScriptLanguageResolver {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageResolver for JavaScriptLanguageResolver {
    fn is_language_name(&self, raw: &str) -> bool {
        is_javascript_language_name(raw)
    }

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        _defaults: Option<&ResolutionDefaults>,
        source: ResolutionSource,
    ) -> Result<ResolvedLanguageSelection, String> {
        let dialect = if let Some(raw) = explicit_dialect {
            JavaScriptDialect::from_extension(raw).ok_or_else(|| {
                format!("Invalid JavaScript dialect '{raw}'. Expected one of: js, jsx, ts, tsx")
            })?
        } else {
            dialect_from_language_name(explicit_language).unwrap_or(JavaScriptDialect::JavaScript)
        };

        Ok(ResolvedLanguageSelection {
            language_key: "JavaScript".to_string(),
            dialect: Some(dialect.as_str().to_string()),
            canonical_label: language_name_for_dialect(dialect),
            source,
            defaulted: false,
        })
    }

    fn resolve_for_explicit_dialect(
        &self,
        explicit_dialect: &str,
        _defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<ResolvedLanguageSelection, String>> {
        JavaScriptDialect::from_extension(explicit_dialect).map(|dialect| {
            Ok(ResolvedLanguageSelection {
                language_key: "JavaScript".to_string(),
                dialect: Some(dialect.as_str().to_string()),
                canonical_label: language_name_for_dialect(dialect),
                source: ResolutionSource::ExplicitLanguage,
                defaulted: false,
            })
        })
    }

    fn resolve_for_extension(
        &self,
        extension: &str,
        _defaults: Option<&ResolutionDefaults>,
        has_engine: &dyn Fn(&str) -> bool,
    ) -> Option<Result<ResolvedLanguageSelection, String>> {
        let js_dialect = JavaScriptDialect::from_extension(extension)?;
        if !has_engine("javascript") {
            return None;
        }

        Some(Ok(ResolvedLanguageSelection {
            language_key: "JavaScript".to_string(),
            dialect: Some(js_dialect.as_str().to_string()),
            canonical_label: language_name_for_dialect(js_dialect),
            source: ResolutionSource::Extension,
            defaulted: false,
        }))
    }

    fn canonicalize_label(&self, raw: &str) -> Option<String> {
        dialect_from_language_name(raw).map(language_name_for_dialect)
    }

    fn expand_filter_labels(&self, query: &str) -> Option<Vec<String>> {
        let normalized = query.trim().to_ascii_lowercase();
        if !is_javascript_language_name(&normalized) {
            return None;
        }

        let js_profiles = ["js", "jsx", "ts", "tsx"];

        if normalized == "javascript" || normalized == "js" {
            let mut labels = vec!["JavaScript/js".to_string()];
            labels.extend(
                js_profiles
                    .iter()
                    .skip(1)
                    .map(|profile| format!("JavaScript/{profile}")),
            );
            return Some(labels);
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if js_profiles.contains(&dialect) {
            return Some(vec![format!("JavaScript/{dialect}")]);
        }

        None
    }
}
