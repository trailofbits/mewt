use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionRequest};

use super::engine::GoLanguageEngine;

pub struct GoLanguageResolver {
    engine: GoLanguageEngine,
}

impl GoLanguageResolver {
    pub fn new() -> Self {
        Self {
            engine: GoLanguageEngine::new(),
        }
    }

    fn is_language_name(raw: &str) -> bool {
        raw.eq_ignore_ascii_case("go") || raw.eq_ignore_ascii_case("golang")
    }
}

impl Default for GoLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for GoLanguageResolver {
    fn family(&self) -> &'static str {
        "go"
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
                return Some(Err("Dialect selection is not supported for Go".to_string()));
            }
            return Some(Ok(&self.engine));
        }

        request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("go"))
            .then_some(Ok(&self.engine as &dyn LanguageEngine))
    }

    fn filter_labels(&self, query: &str) -> Option<Vec<String>> {
        Self::is_language_name(query).then(|| vec![self.engine.canonical_name().to_string()])
    }
}
