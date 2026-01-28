use strum::{Display, EnumString};

#[derive(Debug, Clone)]
pub struct Mutation {
    pub slug: &'static str,
    pub description: &'static str,
    pub severity: MutationSeverity,
}

#[derive(Clone, Debug, Display, EnumString, PartialEq, Eq, Hash)]
pub enum MutationSeverity {
    High,   // eg revert/throw replacement
    Medium, // eg replace line with a comment
    Low,    // eg logic tweaks
}

impl MutationSeverity {
    /// Convert a severity to a numeric value (0 = High, 1 = Medium, 2 = Low)
    pub fn to_numeric(&self) -> usize {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
        }
    }

    /// Get severity from a numeric value
    pub fn from_numeric(value: usize) -> Self {
        match value {
            0 => Self::High,
            1 => Self::Medium,
            _ => Self::Low,
        }
    }
}
