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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::core::resolver::ResolutionRequest;

    use super::*;

    fn request<'a>(
        path: &'a Path,
        explicit_language: Option<&'a str>,
        explicit_dialect: Option<&'a str>,
    ) -> ResolutionRequest<'a> {
        ResolutionRequest {
            path,
            explicit_language,
            explicit_dialect,
            defaults: None,
        }
    }

    #[test]
    fn rust_extension_and_label_resolve_to_single_engine() {
        let resolver = RustLanguageResolver::new();

        let by_extension = resolver
            .resolve(&request(Path::new("src/lib.rs"), None, None))
            .expect("rust resolver should claim .rs")
            .expect(".rs should resolve");
        assert_eq!(by_extension.canonical_name(), "Rust");

        let by_label = resolver
            .resolve(&request(Path::new("__virtual__.txt"), Some("rust"), None))
            .expect("rust resolver should claim rust label")
            .expect("rust label should resolve");
        assert_eq!(by_label.canonical_name(), "Rust");
    }

    #[test]
    fn rust_rejects_explicit_dialect() {
        let resolver = RustLanguageResolver::new();
        let result = resolver
            .resolve(&request(
                Path::new("__virtual__.txt"),
                Some("rust"),
                Some("nightly"),
            ))
            .expect("rust resolver should claim rust label");
        let error = match result {
            Ok(engine) => panic!(
                "expected Rust dialect rejection, got {}",
                engine.canonical_name()
            ),
            Err(error) => error,
        };

        assert_eq!(error, "Dialect selection is not supported for Rust");
    }
}
