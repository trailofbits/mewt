use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionRequest};
use crate::languages::javascript::dialect::{
    JavaScriptDialect, dialect_from_language_name, is_language_name, language_name_for_dialect,
};

use super::engine::JavaScriptDialectEngine;

pub struct JavaScriptLanguageResolver {
    js: JavaScriptDialectEngine,
    jsx: JavaScriptDialectEngine,
    ts: JavaScriptDialectEngine,
    tsx: JavaScriptDialectEngine,
}

impl JavaScriptLanguageResolver {
    pub fn new() -> Self {
        Self {
            js: JavaScriptDialectEngine::new(JavaScriptDialect::JavaScript),
            jsx: JavaScriptDialectEngine::new(JavaScriptDialect::Jsx),
            ts: JavaScriptDialectEngine::new(JavaScriptDialect::TypeScript),
            tsx: JavaScriptDialectEngine::new(JavaScriptDialect::Tsx),
        }
    }

    fn engine_for_dialect(&self, dialect: JavaScriptDialect) -> &dyn LanguageEngine {
        match dialect {
            JavaScriptDialect::JavaScript => &self.js,
            JavaScriptDialect::Jsx => &self.jsx,
            JavaScriptDialect::TypeScript => &self.ts,
            JavaScriptDialect::Tsx => &self.tsx,
        }
    }
}

impl Default for JavaScriptLanguageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageResolver for JavaScriptLanguageResolver {
    fn family(&self) -> &'static str {
        "javascript"
    }

    fn engines(&self) -> Vec<&dyn LanguageEngine> {
        vec![&self.js, &self.jsx, &self.ts, &self.tsx]
    }

    fn accepts_cli_dialect(&self) -> bool {
        false
    }

    fn resolve<'a>(
        &'a self,
        request: &ResolutionRequest<'_>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>> {
        if request.explicit_dialect.is_some() {
            if request.explicit_language.is_some_and(is_language_name) {
                return Some(Err(
                    "javascript does not support --dialect; use .js/.jsx/.ts/.tsx extensions or an explicit javascript/<dialect> label"
                        .to_string(),
                ));
            }
            return None;
        }

        if let Some(explicit_language) = request.explicit_language {
            let dialect = dialect_from_language_name(explicit_language)?;
            return Some(Ok(self.engine_for_dialect(dialect)));
        }

        let dialect = request
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(JavaScriptDialect::from_key)?;
        Some(Ok(self.engine_for_dialect(dialect)))
    }

    fn filter_labels(&self, query: &str) -> Option<Vec<String>> {
        let normalized = query.trim().to_ascii_lowercase();
        if !is_language_name(&normalized) {
            return None;
        }

        if normalized == "javascript" || normalized == "js" {
            return Some(
                JavaScriptDialect::ALL
                    .iter()
                    .map(|dialect| language_name_for_dialect(*dialect))
                    .collect(),
            );
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if let Some(dialect) = JavaScriptDialect::from_key(dialect) {
            return Some(vec![language_name_for_dialect(dialect)]);
        }

        None
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
    fn javascript_extensions_select_concrete_dialect_engines() {
        let resolver = JavaScriptLanguageResolver::new();
        let cases = [
            ("sample.js", "javascript/js"),
            ("sample.jsx", "javascript/jsx"),
            ("sample.ts", "javascript/ts"),
            ("sample.tsx", "javascript/tsx"),
        ];

        for (path, expected) in cases {
            let engine = resolver
                .resolve(&request(Path::new(path), None, None))
                .unwrap_or_else(|| panic!("JavaScript resolver should claim {path}"))
                .expect("extension should resolve");
            assert_eq!(engine.language().to_string(), expected);
        }
    }

    #[test]
    fn javascript_explicit_labels_select_concrete_dialect_engines() {
        let resolver = JavaScriptLanguageResolver::new();
        let cases = [
            ("javascript/js", "javascript/js"),
            ("javascript/jsx", "javascript/jsx"),
            ("javascript/ts", "javascript/ts"),
            ("javascript/tsx", "javascript/tsx"),
        ];

        for (label, expected) in cases {
            let engine = resolver
                .resolve(&request(Path::new("__virtual__.txt"), Some(label), None))
                .unwrap_or_else(|| panic!("JavaScript resolver should claim {label}"))
                .expect("label should resolve");
            assert_eq!(engine.language().to_string(), expected);
        }
    }

    #[test]
    fn javascript_rejects_cli_dialect_for_javascript_labels() {
        let resolver = JavaScriptLanguageResolver::new();
        let result = resolver
            .resolve(&request(
                Path::new("__virtual__.txt"),
                Some("javascript/ts"),
                Some("tsx"),
            ))
            .expect("JavaScript resolver should claim JavaScript label");
        let error = match result {
            Ok(engine) => panic!(
                "expected JavaScript --dialect rejection, got {}",
                engine.language()
            ),
            Err(error) => error,
        };

        assert!(error.contains("javascript does not support --dialect"));
    }

    #[test]
    fn javascript_filter_labels_expand_family_and_concrete_selectors() {
        let resolver = JavaScriptLanguageResolver::new();

        assert_eq!(
            resolver.filter_labels("javascript").unwrap(),
            vec![
                "javascript/js".to_string(),
                "javascript/jsx".to_string(),
                "javascript/ts".to_string(),
                "javascript/tsx".to_string(),
            ]
        );
        assert_eq!(
            resolver.filter_labels("javascript/tsx").unwrap(),
            vec!["javascript/tsx".to_string()]
        );
    }
}
