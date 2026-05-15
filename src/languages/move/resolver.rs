use crate::LanguageEngine;
use crate::core::registry::{LanguageResolver, ResolutionDefaults};
use crate::languages::r#move::dialect::{
    dialect_from_language_name, is_move_language_name, language_name_for_dialect,
};

use super::engine::MoveLanguageEngine;

pub struct MoveLanguageResolver {
    engine: MoveLanguageEngine,
}

impl MoveLanguageResolver {
    pub fn new() -> Self {
        Self {
            engine: MoveLanguageEngine::new(),
        }
    }
}

impl Default for MoveLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for MoveLanguageResolver {
    fn engine(&self) -> &dyn LanguageEngine {
        &self.engine
    }

    fn is_language_name(&self, raw: &str) -> bool {
        is_move_language_name(raw)
    }

    fn resolve_for_explicit_language(
        &self,
        explicit_language: &str,
        explicit_dialect: Option<&str>,
        defaults: Option<&ResolutionDefaults>,
    ) -> Result<String, String> {
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

        Ok(language_name_for_dialect(dialect))
    }

    fn resolve_for_explicit_dialect(
        &self,
        explicit_dialect: &str,
        _defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<String, String>> {
        Some(
            dialect_from_language_name(&format!("move/{explicit_dialect}"))
                .ok_or_else(|| {
                    format!(
                        "Invalid Move dialect '{explicit_dialect}'. Expected one of: sui, iota, aptos"
                    )
                })
                .map(language_name_for_dialect),
        )
    }

    fn resolve_for_extension(
        &self,
        extension: &str,
        defaults: Option<&ResolutionDefaults>,
    ) -> Option<Result<String, String>> {
        if !extension.eq_ignore_ascii_case("move") {
            return None;
        }

        if let Some(defaults) = defaults {
            if let Some(default_move_dialect) = defaults.default_dialects.get("move") {
                if let Some(dialect) =
                    dialect_from_language_name(&format!("move/{}", default_move_dialect.dialect))
                {
                    return Some(Ok(language_name_for_dialect(dialect)));
                }
            }
        }

        Some(Ok("Move/sui".to_string()))
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
            let mut labels = vec!["Move/sui".to_string()];
            labels.extend(
                move_profiles
                    .iter()
                    .skip(1)
                    .map(|profile| format!("Move/{profile}")),
            );
            return Some(labels);
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if move_profiles.contains(&dialect) {
            return Some(vec![format!("Move/{dialect}")]);
        }

        None
    }
}
