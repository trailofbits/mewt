use chrono::DateTime;
use chrono::Utc;
use console::{StyledObject, style};
use serde::Serialize;
use strum::{Display, EnumString};

#[derive(Clone, Debug, Display, EnumString, PartialEq, Serialize)]
#[strum(serialize_all = "PascalCase")]
pub enum Status {
    // bad, mutant is valid
    Uncaught,
    // good, tests caught this mutant
    TestFail,
    // fine, this less severe mutant was skipped because a more severe mutatnt on the same line
    // was uncaught
    Skipped,
    // questionable, tests timed out before passing or failing
    Timeout,
}

impl Status {
    pub fn display(&self) -> StyledObject<String> {
        match &self {
            Status::Uncaught => style(self.to_string()).red().bold(),
            Status::TestFail => style(self.to_string()).green().bold(),
            Status::Timeout => style(self.to_string()).yellow(),
            Status::Skipped => style(self.to_string()).blue(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    pub mutant_id: i64,
    pub status: Status,
    pub output: String,
    pub time: DateTime<Utc>,
    pub duration_ms: u32,
}

/// Summary of a mutation testing campaign
#[derive(Debug, Clone, Default)]
pub struct CampaignSummary {
    /// Total number of mutants tested (not skipped)
    pub tested: usize,
    /// Number of mutants caught by tests
    pub caught: usize,
    /// Number of mutants not caught by tests (test gap)
    pub uncaught: usize,
    /// Number of mutants skipped (due to severity rules)
    pub skipped: usize,
}

impl CampaignSummary {
    // No helper methods needed - uncaught mutants are not errors
}
