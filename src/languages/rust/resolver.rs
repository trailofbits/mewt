use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionRequest};

use super::engine::RustLanguageEngine;

pub struct RustLanguageResolver {
    engine: RustLanguageEngine,
}

impl RustLanguageResolver {
    pub fn new() -> Self {
        Self {
            engine: RustLanguageEngine::new(),
        }
    }

    fn is_language_name(raw: &str) -> bool {
        raw.eq_ignore_ascii_case("rust")
    }
}

impl Default for RustLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for RustLanguageResolver {
    fn family(&self) -> &'static str {
        "rust"
    }

    fn engines(&self) -> Vec<&dyn LanguageEngine> {
        vec![&self.engine]
    }

    fn resolve<'a>(
        &'a self,
        request: &ResolutionRequest<'_>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>> {
        if let Some(explicit_language) = request.explicit_language {
            if !Self::is_language_name(explicit_language) {
                return None;
            }
            if request.explicit_dialect.is_some() {
                return Some(Err(
                    "Dialect selection is not supported for Rust".to_string()
                ));
            }
            return Some(Ok(&self.engine));
        }

        request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
            .then_some(Ok(&self.engine as &dyn LanguageEngine))
    }

    fn filter_labels(&self, query: &str) -> Option<Vec<String>> {
        Self::is_language_name(query).then(|| vec![self.engine.canonical_name().to_string()])
    }
}
