use std::sync::OnceLock;

use tree_sitter::Language as TsLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaScriptDialect {
    JavaScript,
    Jsx,
    TypeScript,
    Tsx,
}

impl JavaScriptDialect {
    pub const ALL: [Self; 4] = [Self::JavaScript, Self::Jsx, Self::TypeScript, Self::Tsx];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::JavaScript => "js",
            Self::Jsx => "jsx",
            Self::TypeScript => "ts",
            Self::Tsx => "tsx",
        }
    }

    pub fn from_key(raw: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|dialect| dialect.as_str().eq_ignore_ascii_case(raw.trim()))
    }
}

pub struct JavaScriptDialectConfig {
    pub dialect: JavaScriptDialect,
}

impl JavaScriptDialectConfig {
    pub fn parser_language(&self) -> &'static TsLanguage {
        match self.dialect {
            JavaScriptDialect::JavaScript | JavaScriptDialect::Jsx => JS_LANGUAGE
                .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_javascript()) }),
            JavaScriptDialect::TypeScript => TS_LANGUAGE
                .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_typescript()) }),
            JavaScriptDialect::Tsx => {
                TSX_LANGUAGE.get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_tsx()) })
            }
        }
    }
}

pub fn language_name_for_dialect(dialect: JavaScriptDialect) -> String {
    format!("javascript/{}", dialect.as_str())
}

pub fn is_language_name(raw: &str) -> bool {
    dialect_from_language_name(raw).is_some()
}

pub fn dialect_from_language_name(raw: &str) -> Option<JavaScriptDialect> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized == "javascript" || normalized == "js" {
        return Some(JavaScriptDialect::JavaScript);
    }

    let dialect = normalized.split_once('/').and_then(|(family, dialect)| {
        (family == "javascript" || family == "js").then_some(dialect)
    })?;

    JavaScriptDialect::from_key(dialect)
}

pub fn config_for_dialect(dialect: JavaScriptDialect) -> JavaScriptDialectConfig {
    JavaScriptDialectConfig { dialect }
}

static JS_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static TS_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static TSX_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_javascript() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_typescript() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_tsx() -> *const tree_sitter::ffi::TSLanguage;
}
