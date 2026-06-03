use crate::LanguageEngine;
use crate::core::resolver::{LanguageResolver, ResolutionRequest};
use crate::languages::javascript::dialect::{
    JavaScriptDialect, dialect_from_language_name, is_javascript_language_name,
    language_name_for_dialect,
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

    fn resolve<'a>(
        &'a self,
        request: &ResolutionRequest<'_>,
    ) -> Option<Result<&'a dyn LanguageEngine, String>> {
        if request.explicit_dialect.is_some() {
            if request
                .explicit_language
                .is_some_and(is_javascript_language_name)
            {
                return Some(Err(
                    "JavaScript does not support --dialect; use .js/.jsx/.ts/.tsx extensions or an explicit JavaScript/<dialect> label"
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
            .and_then(JavaScriptDialect::from_extension)?;
        Some(Ok(self.engine_for_dialect(dialect)))
    }

    fn filter_labels(&self, query: &str) -> Option<Vec<String>> {
        let normalized = query.trim().to_ascii_lowercase();
        if !is_javascript_language_name(&normalized) {
            return None;
        }

        let js_dialects = ["js", "jsx", "ts", "tsx"];

        if normalized == "javascript" || normalized == "js" {
            return Some(
                js_dialects
                    .iter()
                    .map(|dialect| format!("JavaScript/{dialect}"))
                    .collect(),
            );
        }

        let dialect = normalized
            .split_once(['/', ':'])
            .map(|(_, d)| d)
            .unwrap_or_default();

        if js_dialects.contains(&dialect) {
            return Some(vec![language_name_for_dialect(
                JavaScriptDialect::from_extension(dialect)?,
            )]);
        }

        None
    }
}
