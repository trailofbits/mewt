use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionRequest};

use super::engine::DamlLanguageEngine;

pub struct DamlLanguageResolver {
    engine: DamlLanguageEngine,
}

impl DamlLanguageResolver {
    pub fn new() -> Self {
        Self {
            engine: DamlLanguageEngine::new(),
        }
    }

    fn is_language_name(raw: &str) -> bool {
        raw.eq_ignore_ascii_case("daml")
    }
}

impl Default for DamlLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for DamlLanguageResolver {
    fn family(&self) -> &'static str {
        "daml"
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
            return Some(Ok(&self.engine));
        }

        request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("daml"))
            .then_some(Ok(&self.engine as &dyn LanguageEngine))
    }

    fn filter_labels(&self, query: &str) -> Option<Vec<String>> {
        Self::is_language_name(query).then(|| vec![self.engine.language().to_string()])
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::core::resolver::ResolutionRequest;

    use super::*;

    fn request<'a>(path: &'a Path, explicit_language: Option<&'a str>) -> ResolutionRequest<'a> {
        ResolutionRequest {
            path,
            explicit_language,
            defaults: None,
        }
    }

    #[test]
    fn daml_extension_and_label_resolve_to_single_engine() {
        let resolver = DamlLanguageResolver::new();

        let by_extension = resolver
            .resolve(&request(Path::new("template.daml"), None))
            .expect("daml resolver should claim .daml")
            .expect(".daml should resolve");
        assert_eq!(by_extension.language().to_string(), "daml");

        let by_label = resolver
            .resolve(&request(Path::new("__virtual__.txt"), Some("daml")))
            .expect("daml resolver should claim daml label")
            .expect("daml label should resolve");
        assert_eq!(by_label.language().to_string(), "daml");
    }
}
