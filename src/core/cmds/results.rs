use log::info;
use serde::Serialize;
use std::str::FromStr;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::{AppResult, Mutant, MutationSeverity, Outcome, Status, Target};

pub struct ResultsFilters {
    pub target: Option<String>,
    pub verbose: bool,
    pub id: Option<i64>,
    pub all: bool,
    pub status: Option<String>,
    pub language: Option<String>,
    pub mutation_type: Option<String>,
    pub line: Option<u32>,
    pub file: Option<String>,
    pub format: String,
}

// JSON output structures
#[derive(Serialize)]
struct JsonResult {
    mutant: Mutant,
    target: Target,
    outcome: Outcome,
}

#[derive(Serialize)]
struct JsonResults {
    results: Vec<JsonResult>,
}

// SARIF structures (simplified for our use case)
#[derive(Serialize)]
struct SarifReport {
    version: String,
    #[serde(rename = "$schema")]
    schema: String,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: String,
    version: String,
    #[serde(rename = "informationUri")]
    information_uri: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: u32,
}

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
        if *status != Status::Skipped {
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

// Normalize status string to PascalCase using case-insensitive parsing
fn normalize_status(status_str: Option<String>) -> Option<String> {
    status_str.and_then(|s| Status::from_str(&s).ok().map(|status| status.to_string()))
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

pub async fn execute_results(
    store: SqlStore,
    filters: ResultsFilters,
    registry: &LanguageRegistry,
) -> AppResult<()> {
    // Get the data first
    let data = get_results_data(&store, &filters, registry).await?;

    // Handle different output formats
    match filters.format.as_str() {
        "json" => {
            let json_results = JsonResults {
                results: data
                    .iter()
                    .map(|(mutant, target, outcome)| JsonResult {
                        mutant: mutant.clone(),
                        target: target.clone(),
                        outcome: Outcome {
                            mutant_id: outcome.mutant_id,
                            status: outcome.status.clone(),
                            output: outcome.output.clone(),
                            time: outcome.time,
                            duration_ms: outcome.duration_ms,
                        },
                    })
                    .collect(),
            };
            println!("{}", serde_json::to_string_pretty(&json_results)?);
        }
        "sarif" => {
            // Only include uncaught mutants in SARIF (test gaps as warnings)
            let uncaught_results: Vec<SarifResult> = data
                .iter()
                .filter(|(_, _, outcome)| outcome.status == Status::Uncaught)
                .map(|(mutant, target, _)| {
                    let lines = mutant.get_lines();
                    SarifResult {
                        rule_id: mutant.mutation_slug.clone(),
                        level: "warning".to_string(),
                        message: SarifMessage {
                            text: format!(
                                "Uncaught mutant: '{}' -> '{}'",
                                mutant.old_text, mutant.new_text
                            ),
                        },
                        locations: vec![SarifLocation {
                            physical_location: SarifPhysicalLocation {
                                artifact_location: SarifArtifactLocation {
                                    uri: target.path.to_string_lossy().to_string(),
                                },
                                region: SarifRegion {
                                    start_line: lines.0,
                                },
                            },
                        }],
                    }
                })
                .collect();

            let sarif_report = SarifReport {
                version: "2.1.0".to_string(),
                schema: "https://json.schemastore.org/sarif-2.1.0.json".to_string(),
                runs: vec![SarifRun {
                    tool: SarifTool {
                        driver: SarifDriver {
                            name: "mewt".to_string(),
                            version: env!("CARGO_PKG_VERSION").to_string(),
                            information_uri: "https://github.com/trailofbits/mewt".to_string(),
                        },
                    },
                    results: uncaught_results,
                }],
            };
            println!("{}", serde_json::to_string_pretty(&sarif_report)?);
        }
        "ids" => {
            // Just print IDs, one per line
            for (mutant, _, _) in data {
                info!("{}", mutant.id);
            }
        }
        _ => {
            // Default table format
            print_table_format(&data, &filters, &store, registry).await?;
        }
    }

    Ok(())
}

async fn get_results_data(
    store: &SqlStore,
    filters: &ResultsFilters,
    _registry: &LanguageRegistry,
) -> AppResult<Vec<(Mutant, Target, Outcome)>> {
    // If mutant_id is provided, fetch and show only that specific mutant's outcome
    if let Some(id) = filters.id {
        match store.get_mutant(id).await {
            Ok(mutant) => {
                let target = store.get_target(mutant.target_id).await?;
                if let Some(outcome) = store.get_outcome(mutant.id).await? {
                    return Ok(vec![(mutant, target, outcome)]);
                } else {
                    return Ok(vec![]);
                }
            }
            Err(_) => {
                return Ok(vec![]);
            }
        }
    }

    // Use filtered query if any filters are provided
    let use_filters = filters.status.is_some()
        || filters.language.is_some()
        || filters.mutation_type.is_some()
        || filters.line.is_some()
        || filters.file.is_some();

    if use_filters {
        return store
            .get_outcomes_filtered(
                normalize_status(filters.status.clone()),
                filters.language.clone(),
                filters.mutation_type.clone(),
                filters.line,
                filters.file.clone(),
            )
            .await
            .map_err(|e| e.into());
    }

    // Legacy path: no filters, use old logic with target filtering
    let filtered_targets = Target::filter_by_path(store, filters.target.clone()).await?;
    let mut results = Vec::new();

    for target in filtered_targets {
        let mut mutants = store.get_mutants(target.id).await?;
        mutants.sort_by_key(|m| m.byte_offset);

        for mutant in mutants {
            if let Some(outcome) = store.get_outcome(mutant.id).await? {
                // Filter based on flags
                if filters.all || filters.verbose || outcome.status == Status::Uncaught {
                    results.push((mutant, target.clone(), outcome));
                }
            }
        }
    }

    Ok(results)
}

async fn print_table_format(
    data: &[(Mutant, Target, Outcome)],
    filters: &ResultsFilters,
    store: &SqlStore,
    registry: &LanguageRegistry,
) -> AppResult<()> {
    // If mutant_id is provided, special handling
    if filters.id.is_some() {
        if data.is_empty() {
            info!(
                "No outcome found for mutant with ID: {}",
                filters.id.unwrap()
            );
        } else {
            let (mutant, target, outcome) = &data[0];
            info!("Target: {}", target.display());
            print_outcome(mutant, target, outcome, filters.verbose);
        }
        return Ok(());
    }

    // Use filtered query if any filters are provided
    let use_filters = filters.status.is_some()
        || filters.language.is_some()
        || filters.mutation_type.is_some()
        || filters.line.is_some()
        || filters.file.is_some();

    if use_filters {
        if data.is_empty() {
            info!("No outcomes found matching the filters");
            return Ok(());
        }

        // Group by target for display
        let mut by_target: std::collections::HashMap<i64, Vec<&(Mutant, Target, Outcome)>> =
            std::collections::HashMap::new();
        for entry in data {
            by_target.entry(entry.1.id).or_default().push(entry);
        }

        // Display grouped results
        for (_, entries) in by_target {
            if entries.is_empty() {
                continue;
            }
            let target = &entries[0].1;
            info!("Target: {}", target.display());

            for (mutant, target, outcome) in entries {
                print_outcome(mutant, target, outcome, filters.verbose);
            }
            info!(""); // Empty line between targets
        }

        return Ok(());
    }

    // Legacy path: display with per-target statistics
    let filtered_targets = Target::filter_by_path(store, filters.target.clone()).await?;
    if filtered_targets.is_empty() {
        info!("No targets found");
        return Ok(());
    }

    for target in filtered_targets {
        info!("Target: {}", target.display());

        let mut mutants = store.get_mutants(target.id).await?;
        mutants.sort_by_key(|m| m.byte_offset);

        if mutants.is_empty() {
            info!("  No mutants found for this target");
            continue;
        }

        let mut has_outcomes = false;
        let mut overall = OutcomeCounter::new();
        let mut high = OutcomeCounter::new();
        let mut medium = OutcomeCounter::new();
        let mut low = OutcomeCounter::new();

        for mutant in mutants {
            if let Some(outcome) = store.get_outcome(mutant.id).await? {
                let status = outcome.status.clone();
                overall.record(&status);

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

                if filters.verbose || filters.all || status == Status::Uncaught {
                    has_outcomes = true;
                    print_outcome(&mutant, &target, &outcome, filters.verbose);
                }
            }
        }

        if !has_outcomes {
            info!("  No outcomes found for this target");
        }

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
