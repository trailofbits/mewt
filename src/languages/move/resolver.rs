use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionRequest};
use crate::types::config::MoveDialect;

use crate::languages::r#move::dialect::{
    dialect_from_language_name, is_move_language_name, language_name_for_dialect,
};

use super::engine::MoveDialectEngine;

pub struct MoveLanguageResolver {
    sui: MoveDialectEngine,
    iota: MoveDialectEngine,
    aptos: MoveDialectEngine,
}

impl MoveLanguageResolver {
    pub fn new() -> Self {
        Self {
            sui: MoveDialectEngine::new(MoveDialect::Sui),
            iota: MoveDialectEngine::new(MoveDialect::Iota),
            aptos: MoveDialectEngine::new(MoveDialect::Aptos),
        }
    }

    fn engine_for_dialect(&self, dialect: MoveDialect) -> &dyn LanguageEngine {
        match dialect {
            MoveDialect::Sui => &self.sui,
            MoveDialect::Iota => &self.iota,
            MoveDialect::Aptos => &self.aptos,
        }
    }

    fn dialect_from_raw(raw: &str) -> Result<MoveDialect, String> {
        dialect_from_language_name(&format!("move/{raw}")).ok_or_else(|| {
            format!("Invalid Move dialect '{raw}'. Expected one of: sui, iota, aptos")
        })
    }

    fn default_dialect(&self, request: &ResolutionRequest<'_>) -> MoveDialect {
        request
            .defaults
            .and_then(|defaults| defaults.default_dialects.get("move"))
            .and_then(|entry| dialect_from_language_name(&format!("move/{}", entry.dialect)))
            .unwrap_or(MoveDialect::Sui)
    }

    fn resolve_dialect(&self, request: &ResolutionRequest<'_>) -> Result<MoveDialect, String> {
        if let Some(explicit_language) = request.explicit_language {
            if let Some(label_dialect) = dialect_from_language_name(explicit_language) {
                if request.explicit_dialect.is_some()
                    && !explicit_language.eq_ignore_ascii_case("move")
                {
                    return Err(
                        "Use either --language move/<dialect> or --language move --dialect <dialect>, not both"
                            .to_string(),
                    );
                }

                if explicit_language.eq_ignore_ascii_case("move") {
                    if let Some(explicit_dialect) = request.explicit_dialect {
                        return Self::dialect_from_raw(explicit_dialect);
                    }
                    return Ok(self.default_dialect(request));
                }

                return Ok(label_dialect);
            }
        }

        if let Some(explicit_dialect) = request.explicit_dialect {
            return Self::dialect_from_raw(explicit_dialect);
        }

        Ok(self.default_dialect(request))
    }
}

impl Default for MoveLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for MoveLanguageResolver {
    fn family(&self) -> &'static str {
        "move"
    }

    fn engines(&self) -> Vec<&dyn LanguageEngine> {
        vec![&self.sui, &self.iota, &self.aptos]
    }

    fn accepts_cli_dialect(&self) -> bool {
        true
    }

    fn resolve<'a>(
        &'a self,
        request: &ResolutionRequest<'_>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>> {
        if let Some(explicit_language) = request.explicit_language {
            if !is_move_language_name(explicit_language) {
                return None;
            }
            return Some(
                self.resolve_dialect(request)
                    .map(|dialect| self.engine_for_dialect(dialect)),
            );
        }

        if request.explicit_dialect.is_some() {
            return Some(
                self.resolve_dialect(request)
                    .map(|dialect| self.engine_for_dialect(dialect)),
            );
        }

        let is_move_extension = request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("move"));
        if !is_move_extension {
            return None;
        }

        Some(
            self.resolve_dialect(request)
                .map(|dialect| self.engine_for_dialect(dialect)),
        )
    }

    fn filter_labels(&self, query: &str) -> Option<Vec<String>> {
        let normalized = query.trim().to_ascii_lowercase();
        if !is_move_language_name(&normalized) {
            return None;
        }

        let move_dialects = ["sui", "iota", "aptos"];

        if normalized == "move" {
            return Some(
                move_dialects
                    .iter()
                    .map(|dialect| format!("Move/{dialect}"))
                    .collect(),
            );
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if move_dialects.contains(&dialect) {
            return Some(vec![language_name_for_dialect(dialect_from_language_name(
                &format!("move/{dialect}"),
            )?)]);
        }

        None
    }
}
