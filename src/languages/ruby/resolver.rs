use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionRequest};

use super::engine::RubyLanguageEngine;

pub struct RubyLanguageResolver {
    engine: RubyLanguageEngine,
}

impl RubyLanguageResolver {
    pub fn new() -> Self {
        Self {
            engine: RubyLanguageEngine::new(),
        }
    }

    fn is_language_name(raw: &str) -> bool {
        raw.eq_ignore_ascii_case("ruby") || raw.eq_ignore_ascii_case("rb")
    }
}

impl Default for RubyLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for RubyLanguageResolver {
    fn family(&self) -> &'static str {
        "ruby"
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
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rb"))
            .then_some(Ok(&self.engine as &dyn LanguageEngine))
    }

    fn filter_labels(&self, query: &str) -> Option<Vec<String>> {
        Self::is_language_name(query).then(|| vec![self.engine.language().to_string()])
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::core::resolver::ResolutionRequest;

    fn resolver() -> RubyLanguageResolver {
        RubyLanguageResolver::new()
    }

    fn request<'a>(path: &'a Path, explicit_language: Option<&'a str>) -> ResolutionRequest<'a> {
        ResolutionRequest {
            path,
            explicit_language,
            defaults: None,
        }
    }

    #[test]
    fn resolves_explicit_ruby() {
        assert!(
            resolver()
                .resolve(&request(Path::new("irrelevant.txt"), Some("ruby")))
                .is_some()
        );
    }

    #[test]
    fn resolves_explicit_rb_alias() {
        assert!(
            resolver()
                .resolve(&request(Path::new("irrelevant.txt"), Some("rb")))
                .is_some()
        );
    }

    #[test]
    fn resolves_explicit_case_insensitive() {
        assert!(
            resolver()
                .resolve(&request(Path::new("irrelevant.txt"), Some("Ruby")))
                .is_some()
        );
        assert!(
            resolver()
                .resolve(&request(Path::new("irrelevant.txt"), Some("RUBY")))
                .is_some()
        );
    }

    #[test]
    fn resolves_rb_extension() {
        assert!(
            resolver()
                .resolve(&request(Path::new("foo.rb"), None))
                .is_some()
        );
    }

    #[test]
    fn does_not_resolve_other_extension() {
        assert!(
            resolver()
                .resolve(&request(Path::new("foo.py"), None))
                .is_none()
        );
    }

    #[test]
    fn does_not_resolve_other_language() {
        assert!(
            resolver()
                .resolve(&request(Path::new("foo.rb"), Some("python")))
                .is_none()
        );
    }

    #[test]
    fn filter_labels_returns_ruby_for_ruby() {
        let labels = resolver().filter_labels("ruby").unwrap();
        assert_eq!(labels, vec!["ruby".to_string()]);
    }

    #[test]
    fn filter_labels_returns_none_for_other() {
        assert!(resolver().filter_labels("python").is_none());
    }
}
