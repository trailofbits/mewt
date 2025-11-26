use log::info;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::{AppResult, Mutant, MutationSeverity, Outcome, Status, Target};

// Simple helper to track caught/eligible per severity (and overall)
struct OutcomeCounter {
    eligible: u32,
    caught: u32,
}

impl OutcomeCounter {
    fn new() -> Self {
        Self {
            eligible: 0,
            caught: 0,
        }
    }
    fn record(&mut self, status: &Status) {
        if *status != Status::Skipped && *status != Status::BuildFail {
            self.eligible += 1;
            if *status == Status::TestFail {
                self.caught += 1;
            }
        }
    }
    fn percent_caught(&self) -> f64 {
        if self.eligible > 0 {
            (self.caught as f64 / self.eligible as f64) * 100.0
        } else {
            0.0
        }
    }
}

// Print outcome details and verbose information if requested
fn print_outcome(mutant: &Mutant, target: &Target, outcome: &Outcome, verbose: bool) {
    info!(
        "  {:<9} | {}",
        &outcome.status.display(),
        mutant.display(target)
    );

    // Print output & timing info if verbose
    if verbose {
        info!(
            "  Executed at: {}, Duration: {}ms",
            outcome.time, outcome.duration_ms
        );
        if !outcome.output.is_empty() {
            info!(
                "{}",
                outcome
                    .output
                    .trim()
                    .lines()
                    .map(|line| format!("  {line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }
}

pub async fn execute(
    store: SqlStore,
    target_path: Option<String>,
    verbose: bool,
    mutant_id: Option<i64>,
    all: bool,
    registry: &LanguageRegistry,
) -> AppResult<()> {
    // Get targets filtered by path
    let filtered_targets = Target::filter_by_path(&store, target_path.clone()).await?;
    if filtered_targets.is_empty() {
        info!("No targets found");
        return Ok(());
    }

    // If mutant_id is provided, fetch and show only that specific mutant's outcome
    if let Some(id) = mutant_id {
        // Get the mutant
        match store.get_mutant(id).await {
            Ok(mutant) => {
                // Get the target for this mutant
                let target = store.get_target(mutant.target_id).await?;
                info!("Target: {}", target.display());

                // Get the outcome for this mutant
                if let Some(outcome) = store.get_outcome(mutant.id).await? {
                    print_outcome(&mutant, &target, &outcome, verbose);
                } else {
                    info!("  No outcome found for mutant with ID: {id}");
                }
            }
            Err(_) => {
                info!("Mutant with ID {id} not found");
            }
        }
        return Ok(());
    }

    // Group outcomes by target
    for target in filtered_targets {
        info!("Target: {}", target.display());

        // Get all mutants for this target
        let mut mutants = store.get_mutants(target.id).await?;
        // Sort by byte_offset so earlier mutations in the file come first
        mutants.sort_by_key(|m| m.byte_offset);
        if mutants.is_empty() {
            info!("  No mutants found for this target");
            continue;
        }

        // Retrieve outcomes for each mutant
        let mut has_outcomes = false;
        // Overall and per-severity tallies
        let mut overall = OutcomeCounter::new();
        let mut high = OutcomeCounter::new();
        let mut medium = OutcomeCounter::new();
        let mut low = OutcomeCounter::new();
        for mutant in mutants {
            // Get the outcome for this mutant
            if let Some(outcome) = store.get_outcome(mutant.id).await? {
                // Update counters first
                let status = outcome.status.clone();
                overall.record(&status);
                // Severity buckets via mutation severity lookup
                let severity = registry
                    .get_engine(&target.language)
                    .unwrap()
                    .get_severity_by_slug(&mutant.mutation_slug)
                    .unwrap_or(MutationSeverity::Low);
                match severity {
                    MutationSeverity::High => high.record(&status),
                    MutationSeverity::Medium => medium.record(&status),
                    MutationSeverity::Low => low.record(&status),
                };

                // Show all outcomes if verbose, all flag is set, or only uncaught outcomes otherwise
                if verbose || all || status == Status::Uncaught {
                    has_outcomes = true;
                    print_outcome(&mutant, &target, &outcome, verbose);
                }
            }
        }

        if !has_outcomes {
            info!("  No outcomes found for this target");
        }

        // Print per-severity caught/missed lines
        info!(
            "High severity caught: {:.1}% ({} / {})",
            high.percent_caught(),
            high.caught,
            high.eligible
        );
        info!(
            "Medium severity caught: {:.1}% ({} / {})",
            medium.percent_caught(),
            medium.caught,
            medium.eligible
        );
        info!(
            "Low severity caught: {:.1}% ({} / {})",
            low.percent_caught(),
            low.caught,
            low.eligible
        );
        info!(
            "Total caught: {:.1}% ({} / {})",
            overall.percent_caught(),
            overall.caught,
            overall.eligible
        );
        info!(""); // Empty line between targets
    }

    Ok(())
}
