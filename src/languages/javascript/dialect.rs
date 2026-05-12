use std::path::Path;
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
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::JavaScript => "js",
            Self::Jsx => "jsx",
            Self::TypeScript => "ts",
            Self::Tsx => "tsx",
        }
    }
}

pub struct JavaScriptDialectProfile {
    pub dialect: JavaScriptDialect,
}

impl JavaScriptDialectProfile {
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

pub fn profile_for_target_path(path: &Path) -> JavaScriptDialectProfile {
    let dialect = match path.extension().and_then(|e| e.to_str()) {
        Some("ts") => JavaScriptDialect::TypeScript,
        Some("tsx") => JavaScriptDialect::Tsx,
        Some("jsx") => JavaScriptDialect::Jsx,
        _ => JavaScriptDialect::JavaScript,
    };

    JavaScriptDialectProfile { dialect }
}

static JS_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static TS_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static TSX_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_javascript() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_typescript() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_tsx() -> *const tree_sitter::ffi::TSLanguage;
}
