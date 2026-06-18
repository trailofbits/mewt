use crate::LanguageEngine;
use crate::core::resolver::{DialectPolicy, LanguageResolver, ResolutionRequest};

use crate::languages::r#move::dialect::{
    MoveDialect, dialect_from_language_name, is_language_name, language_name_for_dialect,
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
        MoveDialect::from_key(raw).ok_or_else(|| {
            let expected = MoveDialect::ALL
                .iter()
                .map(MoveDialect::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            format!("Invalid Move dialect '{raw}'. Expected one of: {expected}")
        })
    }

    fn default_dialect(&self, request: &ResolutionRequest<'_>) -> Result<MoveDialect, String> {
        let Some(entry) = request
            .defaults
            .and_then(|defaults| defaults.default_dialects.get("move"))
        else {
            return Ok(MoveDialect::Sui);
        };

        Self::dialect_from_raw(&entry.dialect)
    }

    fn resolve_dialect(&self, request: &ResolutionRequest<'_>) -> Result<MoveDialect, String> {
        if let Some(explicit_language) = request.explicit_language {
            if let Some(label_dialect) = dialect_from_language_name(explicit_language) {
                if explicit_language.eq_ignore_ascii_case("move") {
                    return self.default_dialect(request);
                }

                return Ok(label_dialect);
            }
        }

        self.default_dialect(request)
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

    fn dialect_policy(&self) -> DialectPolicy {
        DialectPolicy {
            dialects: &["sui", "iota", "aptos"],
        }
    }

    fn resolve<'a>(
        &'a self,
        request: &ResolutionRequest<'_>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>> {
        if let Some(explicit_language) = request.explicit_language {
            if !is_language_name(explicit_language) {
                return None;
            }
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
        if !is_language_name(&normalized) {
            return None;
        }

        if normalized == "move" {
            return Some(
                MoveDialect::ALL
                    .iter()
                    .map(|dialect| language_name_for_dialect(*dialect))
                    .collect(),
            );
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if let Some(dialect) = MoveDialect::from_key(dialect) {
            return Some(vec![language_name_for_dialect(dialect)]);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::core::resolver::{DialectDefault, ResolutionDefaults, ResolutionRequest};

    use super::*;

    fn request<'a>(
        path: &'a Path,
        explicit_language: Option<&'a str>,
        defaults: Option<&'a ResolutionDefaults>,
    ) -> ResolutionRequest<'a> {
        ResolutionRequest {
            path,
            explicit_language,
            defaults,
        }
    }

    fn defaults_for_move(dialect: &str) -> ResolutionDefaults {
        let mut defaults = ResolutionDefaults::default();
        defaults.default_dialects.insert(
            "move".to_string(),
            DialectDefault {
                dialect: dialect.to_string(),
            },
        );
        defaults
    }

    #[test]
    fn move_extension_uses_configured_dialect_default() {
        let resolver = MoveLanguageResolver::new();
        let defaults = defaults_for_move("aptos");
        let engine = resolver
            .resolve(&request(Path::new("module.move"), None, Some(&defaults)))
            .expect("move resolver should claim .move")
            .expect("valid configured dialect");

        assert_eq!(engine.language().to_string(), "move/aptos");
    }

    #[test]
    fn bare_move_language_uses_configured_dialect_default() {
        let resolver = MoveLanguageResolver::new();
        let defaults = defaults_for_move("iota");
        let engine = resolver
            .resolve(&request(
                Path::new("__virtual__.txt"),
                Some("move"),
                Some(&defaults),
            ))
            .expect("move resolver should claim bare move label")
            .expect("valid configured dialect");

        assert_eq!(engine.language().to_string(), "move/iota");
    }

    #[test]
    fn invalid_configured_move_dialect_errors_in_move_resolver() {
        let resolver = MoveLanguageResolver::new();
        let defaults = defaults_for_move("unknown");
        let result = resolver
            .resolve(&request(Path::new("module.move"), None, Some(&defaults)))
            .expect("move resolver should claim .move");
        let error = match result {
            Ok(engine) => panic!(
                "expected invalid configured dialect to fail, got {}",
                engine.language()
            ),
            Err(error) => error,
        };

        assert!(error.contains("Invalid Move dialect 'unknown'"));
    }
}
