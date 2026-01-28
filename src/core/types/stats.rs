use std::collections::HashMap;

/// Statistics for a specific target
#[derive(Debug)]
pub struct TargetStats {
    pub total_mutants: usize,
    pub tested: usize,
    pub untested: usize,
    pub caught: usize,
    pub uncaught: usize,
    pub timeout: usize,
    pub skipped: usize,
    pub build_fail: usize,
    /// Map from mutation_slug to (eligible_count, caught_count)
    pub severity_stats: HashMap<String, (usize, usize)>,
}

/// Campaign-wide severity statistics
#[derive(Debug)]
pub struct CampaignSeverityStats {
    /// Map from mutation_slug to (eligible_count, caught_count)
    pub severity_stats: HashMap<String, (usize, usize)>,
}
