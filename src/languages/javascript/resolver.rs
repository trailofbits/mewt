use crate::LanguageEngine;
use crate::core::resolver::{DialectPolicy, LanguageResolver, ResolutionRequest};
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

    fn dialect_policy(&self) -> DialectPolicy {
        DialectPolicy {
            dialects: &["js", "jsx", "ts", "tsx"],
        }
    }

    fn resolve<'a>(
        &'a self,
        request: &ResolutionRequest<'_>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>> {
        if let Some(explicit_language) = request.explicit_language {
            let label_dialect = dialect_from_language_name(explicit_language)?;

            if !explicit_language.eq_ignore_ascii_case("javascript")
                && !explicit_language.eq_ignore_ascii_case("js")
            {
                return Some(Ok(self.engine_for_dialect(label_dialect)));
            }
        }

        let dialect = request
            .defaults
            .and_then(|defaults| defaults.default_dialects.get("javascript"))
            .and_then(|entry| JavaScriptDialect::from_key(&entry.dialect))
            .or_else(|| {
                request
                    .path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(JavaScriptDialect::from_key)
            })?;
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

    fn defaults_for_javascript(dialect: &str) -> ResolutionDefaults {
        let mut defaults = ResolutionDefaults::default();
        defaults.default_dialects.insert(
            "javascript".to_string(),
            DialectDefault {
                dialect: dialect.to_string(),
            },
        );
        defaults
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
    fn bare_javascript_label_uses_extension_or_configured_dialect() {
        let resolver = JavaScriptLanguageResolver::new();

        let by_extension = resolver
            .resolve(&request(Path::new("sample.tsx"), Some("javascript"), None))
            .expect("JavaScript resolver should claim bare JavaScript label")
            .expect("extension should select concrete dialect");
        assert_eq!(by_extension.language().to_string(), "javascript/tsx");

        let defaults = defaults_for_javascript("jsx");
        let by_config = resolver
            .resolve(&request(
                Path::new("sample.js"),
                Some("javascript"),
                Some(&defaults),
            ))
            .expect("JavaScript resolver should claim bare JavaScript label")
            .expect("config should select concrete dialect");
        assert_eq!(by_config.language().to_string(), "javascript/jsx");
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
