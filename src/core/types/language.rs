use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Language {
    pub family: String,
    pub dialect: Option<String>,
}

impl Language {
    pub fn new(
        family: impl Into<String>,
        dialect: Option<impl Into<String>>,
    ) -> Result<Self, String> {
        let family = family.into();
        let dialect = dialect.map(Into::into);
        Self::validate_part("language family", &family)?;
        if let Some(dialect) = &dialect {
            Self::validate_part("language dialect", dialect)?;
        }
        Ok(Self {
            family: family.to_ascii_lowercase(),
            dialect: dialect.map(|dialect| dialect.to_ascii_lowercase()),
        })
    }

    pub fn family(&self) -> &str {
        &self.family
    }

    pub fn dialect(&self) -> Option<&str> {
        self.dialect.as_deref()
    }

    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        match (self.dialect(), other.split_once('/')) {
            (Some(dialect), Some((family, other_dialect))) => {
                self.family().eq_ignore_ascii_case(family)
                    && dialect.eq_ignore_ascii_case(other_dialect)
            }
            (None, None) => self.family().eq_ignore_ascii_case(other),
            _ => false,
        }
    }

    fn validate_part(label: &str, part: &str) -> Result<(), String> {
        if part.is_empty() {
            return Err(format!("{label} cannot be empty"));
        }
        if part.trim() != part {
            return Err(format!(
                "{label} cannot contain leading or trailing whitespace"
            ));
        }
        if part.contains('/') {
            return Err(format!("{label} cannot contain '/'"));
        }
        if part.contains(':') {
            return Err(format!("{label} cannot contain ':'"));
        }
        Ok(())
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.dialect {
            Some(dialect) => write!(f, "{}/{}", self.family, dialect),
            None => f.write_str(&self.family),
        }
    }
}

impl FromStr for Language {
    type Err = String;
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let mut parts = raw.split('/');
        let family = parts
            .next()
            .ok_or_else(|| "language family cannot be empty".to_string())?;
        let dialect = parts.next();
        if parts.next().is_some() {
            return Err("language can contain at most one '/' separator".to_string());
        }
        Self::new(family, dialect)
    }
}

impl PartialEq<str> for Language {
    fn eq(&self, other: &str) -> bool {
        match (self.dialect(), other.split_once('/')) {
            (Some(dialect), Some((family, other_dialect))) => {
                self.family() == family && dialect == other_dialect
            }
            (None, None) => self.family() == other,
            _ => false,
        }
    }
}

impl PartialEq<&str> for Language {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_family_only_language() {
        let language: Language = "Rust".parse().unwrap();
        assert_eq!(language.family(), "rust");
        assert_eq!(language.dialect(), None);
        assert_eq!(language.to_string(), "rust");
    }

    #[test]
    fn parses_family_and_dialect_language() {
        let language: Language = "Move/sui".parse().unwrap();
        assert_eq!(language.family(), "move");
        assert_eq!(language.dialect(), Some("sui"));
        assert_eq!(language.to_string(), "move/sui");
    }

    #[test]
    fn rejects_ambiguous_or_empty_language_strings() {
        for raw in ["", " Move", "Move ", "Move/", "/sui", "move/sui/extra"] {
            assert!(raw.parse::<Language>().is_err(), "{raw:?} should fail");
        }
    }
}
