use crate::core::registry::{
    LanguageResolver, ResolutionDefaults, ResolutionSource, ResolvedLanguageSelection,
};
use crate::languages::r#move::dialect::{
    dialect_from_language_name, is_move_language_name, language_name_for_dialect,
};

pub struct MoveLanguageResolver;

impl MoveLanguageResolver {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageResolver for MoveLanguageResolver {
    fn is_language_name(&self, raw: &str) -> bool {
        is_move_language_name(raw)
    }

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&ResolutionDefaults>,
        source: ResolutionSource,
    ) -> Result<ResolvedLanguageSelection, String> {
        let dialect = if let Some(raw) = explicit_dialect {
            dialect_from_language_name(&format!("move/{raw}")).ok_or_else(|| {
                format!("Invalid Move dialect '{raw}'. Expected one of: sui, iota, aptos")
            })?
        } else if let Some(from_label) = dialect_from_language_name(explicit_language) {
            from_label
        } else if let Some(defaults) = defaults {
            defaults
                .default_dialects
                .get("move")
                .and_then(|entry| dialect_from_language_name(&format!("move/{}", entry.dialect)))
                .unwrap_or(crate::types::config::MoveDialect::Sui)
        } else {
            crate::types::config::MoveDialect::Sui
        };

        let defaulted = explicit_dialect.is_none()
            && defaults
                .and_then(|d| d.default_dialects.get("move"))
                .is_some_and(|entry| entry.defaulted);

        Ok(ResolvedLanguageSelection {
            language_key: "Move".to_string(),
            dialect: Some(dialect.as_str().to_string()),
            canonical_label: language_name_for_dialect(dialect),
            source,
            defaulted,
        })
    }

    fn resolve_for_explicit_dialect(
        &self,
        explicit_dialect: &str,
        _defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<ResolvedLanguageSelection, String>> {
        Some(
            dialect_from_language_name(&format!("move/{explicit_dialect}"))
                .ok_or_else(|| {
                    format!(
                        "Invalid Move dialect '{explicit_dialect}'. Expected one of: sui, iota, aptos"
                    )
                })
                .map(|dialect| ResolvedLanguageSelection {
                    language_key: "Move".to_string(),
                    dialect: Some(dialect.as_str().to_string()),
                    canonical_label: language_name_for_dialect(dialect),
                    source: ResolutionSource::ExplicitLanguage,
                    defaulted: false,
                }),
        )
    }

    fn resolve_for_extension(
        &self,
        extension: &str,
        defaults: Option<&ResolutionDefaults>,
        _has_engine: &dyn Fn(&str) -> bool,
    ) -> Option<Result<ResolvedLanguageSelection, String>> {
        if !extension.eq_ignore_ascii_case("move") {
            return None;
        }

        if let Some(defaults) = defaults {
            if let Some(default_move_dialect) = defaults.default_dialects.get("move") {
                if let Some(dialect) =
                    dialect_from_language_name(&format!("move/{}", default_move_dialect.dialect))
                {
                    return Some(Ok(ResolvedLanguageSelection {
                        language_key: "Move".to_string(),
                        dialect: Some(dialect.as_str().to_string()),
                        canonical_label: language_name_for_dialect(dialect),
                        source: ResolutionSource::Extension,
                        defaulted: default_move_dialect.defaulted,
                    }));
                }
            }
        }

        None
    }

    fn canonicalize_label(&self, raw: &str) -> Option<String> {
        dialect_from_language_name(raw).map(language_name_for_dialect)
    }

    fn expand_filter_labels(&self, query: &str) -> Option<Vec<String>> {
        let normalized = query.trim().to_ascii_lowercase();
        if !is_move_language_name(&normalized) {
            return None;
        }

        let move_profiles = ["sui", "iota", "aptos"];

        if normalized == "move" {
            let mut labels = vec!["Move".to_string()];
            labels.extend(
                move_profiles
                    .iter()
                    .map(|profile| format!("Move/{profile}")),
            );
            return Some(labels);
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if move_profiles.contains(&dialect) {
            if dialect == "sui" {
                return Some(vec!["Move/sui".to_string(), "Move".to_string()]);
            }
            return Some(vec![format!("Move/{dialect}")]);
        }

        None
    }
}
