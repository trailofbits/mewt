use std::sync::OnceLock;

use tree_sitter::Language as TsLanguage;

use crate::types::config::MoveDialect;

#[derive(Debug, Clone, Copy)]
pub struct MoveDialectProfile {
    pub dialect: MoveDialect,
    pub abort_statement: &'static str,
    unsupported_mutation_slugs: &'static [&'static str],
}

impl MoveDialectProfile {
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

pub fn profile_for_target_language(language_name: &str) -> MoveDialectProfile {
    let dialect = dialect_from_language_name(language_name).unwrap_or(MoveDialect::Sui);
    profile_for_dialect(dialect)
}

pub fn profile_for_dialect(dialect: MoveDialect) -> MoveDialectProfile {
    match dialect {
        MoveDialect::Sui => MoveDialectProfile {
            dialect,
            abort_statement: "abort 0;",
            // Move has no compound assignment operators in this parser profile.
            unsupported_mutation_slugs: &["AAOS", "BAOS", "SAOS"],
        },
        MoveDialect::Iota => MoveDialectProfile {
            dialect,
            abort_statement: "abort 0;",
            // Start with same capabilities as Sui until grammar-specific deltas are added.
            unsupported_mutation_slugs: &["AAOS", "BAOS", "SAOS"],
        },
        MoveDialect::Aptos => MoveDialectProfile {
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

pub fn validate_source_for_dialect(source: &str, dialect: MoveDialect) -> Result<(), String> {
    if matches!(dialect, MoveDialect::Iota) && source.contains("public(package)") {
        return Err(
            "Construct 'public(package)' is treated as Sui-only and is not supported under Move/iota"
                .to_string(),
        );
    }

    if matches!(dialect, MoveDialect::Sui) && source.contains("@iota_only") {
        return Err(
            "Construct marker '@iota_only' requires Move/iota and is not supported under Move/sui"
                .to_string(),
        );
    }

    if !matches!(dialect, MoveDialect::Aptos) && source.contains("@aptos_only") {
        return Err(
            "Construct marker '@aptos_only' requires Move/aptos and is not supported under this dialect"
                .to_string(),
        );
    }

    Ok(())
}

pub fn language_name_for_dialect(dialect: MoveDialect) -> String {
    format!("Move/{}", dialect.as_str())
}

pub fn dialect_from_language_name(language_name: &str) -> Option<MoveDialect> {
    let normalized = language_name.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "move" | "move/sui" | "move:sui" => Some(MoveDialect::Sui),
        "move/iota" | "move:iota" => Some(MoveDialect::Iota),
        "move/aptos" | "move:aptos" => Some(MoveDialect::Aptos),
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
    fn parses_canonical_and_profiled_move_names() {
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
        assert_eq!(dialect_from_language_name("move: iota"), None);
    }

    #[test]
    fn emits_profiled_language_names() {
        assert_eq!(language_name_for_dialect(MoveDialect::Sui), "Move/sui");
        assert_eq!(language_name_for_dialect(MoveDialect::Iota), "Move/iota");
        assert_eq!(language_name_for_dialect(MoveDialect::Aptos), "Move/aptos");
    }

    #[test]
    fn rejects_sui_only_construct_for_iota() {
        let err = validate_source_for_dialect("public(package) fun f() {}", MoveDialect::Iota)
            .expect_err("Sui-only construct should fail under iota");
        assert!(err.contains("Sui-only"));
    }

    #[test]
    fn rejects_iota_sensitive_marker_for_sui() {
        let err = validate_source_for_dialect("// @iota_only", MoveDialect::Sui)
            .expect_err("iota marker should fail under sui");
        assert!(err.contains("Move/iota"));
    }

    #[test]
    fn rejects_aptos_sensitive_marker_for_non_aptos() {
        let err = validate_source_for_dialect("// @aptos_only", MoveDialect::Sui)
            .expect_err("aptos marker should fail under sui");
        assert!(err.contains("Move/aptos"));

        assert!(validate_source_for_dialect("// @aptos_only", MoveDialect::Aptos).is_ok());
    }
}
