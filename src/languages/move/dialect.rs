use std::sync::OnceLock;

use tree_sitter::Language as TsLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDialect {
    Sui,
    Iota,
    Aptos,
}

impl MoveDialect {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sui => "sui",
            Self::Iota => "iota",
            Self::Aptos => "aptos",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MoveDialectConfig {
    pub dialect: MoveDialect,
    pub abort_statement: &'static str,
    unsupported_mutation_slugs: &'static [&'static str],
}

impl MoveDialectConfig {
    pub fn parser_language(&self) -> &'static TsLanguage {
        match self.dialect {
            MoveDialect::Sui => SUI_MOVE_LANGUAGE
                .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_move_sui()) }),
            MoveDialect::Iota => IOTA_MOVE_LANGUAGE
                .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_move_iota()) }),
            MoveDialect::Aptos => APTOS_MOVE_LANGUAGE
                .get_or_init(|| unsafe { TsLanguage::from_raw(tree_sitter_move_on_aptos()) }),
        }
    }

    pub fn supports_mutation_slug(&self, slug: &str) -> bool {
        !self.unsupported_mutation_slugs.contains(&slug)
    }
}

pub fn config_for_target_language(language_name: &str) -> MoveDialectConfig {
    let dialect = dialect_from_language_name(language_name).unwrap_or(MoveDialect::Sui);
    config_for_dialect(dialect)
}

pub fn config_for_dialect(dialect: MoveDialect) -> MoveDialectConfig {
    match dialect {
        MoveDialect::Sui => MoveDialectConfig {
            dialect,
            abort_statement: "abort 0;",
            // Move has no compound assignment operators in this parser dialect.
            unsupported_mutation_slugs: &["AAOS", "BAOS", "SAOS"],
        },
        MoveDialect::Iota => MoveDialectConfig {
            dialect,
            abort_statement: "abort 0;",
            // Start with same capabilities as Sui until grammar-specific deltas are added.
            unsupported_mutation_slugs: &["AAOS", "BAOS", "SAOS"],
        },
        MoveDialect::Aptos => MoveDialectConfig {
            dialect,
            abort_statement: "abort 0;",
            // Start with same capabilities as Sui until grammar-specific deltas are added.
            unsupported_mutation_slugs: &["AAOS", "BAOS", "SAOS"],
        },
    }
}

pub fn is_move_language_name(language_name: &str) -> bool {
    dialect_from_language_name(language_name).is_some()
}

pub fn language_name_for_dialect(dialect: MoveDialect) -> String {
    format!("Move/{}", dialect.as_str())
}

pub fn dialect_from_language_name(language_name: &str) -> Option<MoveDialect> {
    let normalized = language_name.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "move" | "move/sui" => Some(MoveDialect::Sui),
        "move/iota" => Some(MoveDialect::Iota),
        "move/aptos" => Some(MoveDialect::Aptos),
        _ => None,
    }
}

static SUI_MOVE_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static IOTA_MOVE_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();
static APTOS_MOVE_LANGUAGE: OnceLock<TsLanguage> = OnceLock::new();

unsafe extern "C" {
    fn tree_sitter_move_sui() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_move_iota() -> *const tree_sitter::ffi::TSLanguage;
    fn tree_sitter_move_on_aptos() -> *const tree_sitter::ffi::TSLanguage;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_and_dialect_move_names() {
        assert_eq!(dialect_from_language_name("Move"), Some(MoveDialect::Sui));
        assert_eq!(
            dialect_from_language_name("move/iota"),
            Some(MoveDialect::Iota)
        );
        assert_eq!(
            dialect_from_language_name("move/aptos"),
            Some(MoveDialect::Aptos)
        );
        assert_eq!(dialect_from_language_name("suimove"), None);
        assert_eq!(dialect_from_language_name("move:iota"), None);
        assert_eq!(dialect_from_language_name("move: iota"), None);
    }

    #[test]
    fn emits_dialect_language_names() {
        assert_eq!(language_name_for_dialect(MoveDialect::Sui), "Move/sui");
        assert_eq!(language_name_for_dialect(MoveDialect::Iota), "Move/iota");
        assert_eq!(language_name_for_dialect(MoveDialect::Aptos), "Move/aptos");
    }
}
