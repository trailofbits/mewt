use log::{info, warn};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

use crate::core::prioritize::{self, Annotation, Purpose};

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::utils::parse_csv;
use crate::types::{AppResult, Mutant, MutationSeverity, Outcome, Status, Target};

pub struct ResultsFilters {
    pub target: Option<String>,
    pub verbose: bool,
    pub id: Option<i64>,
    pub all: bool,
    pub status: Option<String>,
    pub language: Option<String>,
    pub mutation_types: Option<String>,
    pub severity: Option<String>,
    pub line: Option<u32>,
    pub format: String,
}

// JSON output structures
#[derive(Serialize)]
struct JsonResult {
    mutant: Mutant,
    target: Target,
    outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    test_goal_priority: Option<Annotation>,
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
    use std::str::FromStr;
    status_str.and_then(|s| Status::from_str(&s).ok().map(|status| status.to_string()))
}

// Print outcome details and verbose information if requested
fn print_outcome(
    mutant: &Mutant,
    target: &Target,
    outcome: &Outcome,
    verbose: bool,
    priority: Option<&Annotation>,
) {
    info!(
        "  {:<9} | {}",
        outcome.status.display(),
        mutant.display(target)
    );

    if let Some(priority) = priority {
        info!(
            "    Test-goal priority: {:.2}/4 (distribution confidence {:.2}{}) [heuristic]",
            priority.score,
            priority.confidence,
            priority
                .category
                .as_deref()
                .map(|s| format!(", {s}"))
                .unwrap_or_default()
        );
    }

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
    let mut priorities = HashMap::new();
    // IDs and SARIF keep their exact existing output semantics.
    if !matches!(filters.format.as_str(), "ids" | "sarif")
        && data.iter().any(|(_, _, o)| o.status == Status::Uncaught)
    {
        let targets: BTreeMap<i64, &Target> = data
            .iter()
            .filter(|(_, _, o)| o.status == Status::Uncaught)
            .map(|(_, t, _)| (t.id, t))
            .collect();
        for target in targets.values() {
            match prioritize::prepare(&store, target, Purpose::Post).await {
                Ok(plan) => match prioritize::annotations(&store, target, &plan).await {
                    Ok(found) => priorities.extend(found),
                    Err(e) => {
                        warn!("Cannot read test-goal annotations: {e}; showing unannotated results")
                    }
                },
                Err(e) => {
                    warn!("Cannot prepare test-goal annotations: {e}; showing unannotated results")
                }
            }
        }
    }

    // Handle different output formats
    match filters.format.as_str() {
        "json" => {
            let json_results = JsonResults {
                results: data
                    .iter()
                    .map(|(mutant, target, outcome)| JsonResult {
                        test_goal_priority: if outcome.status == Status::Uncaught {
                            priorities.get(&mutant.id).cloned()
                        } else {
                            None
                        },
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
            print_table_format(&data, &filters, registry, &priorities).await?;
        }
    }

    Ok(())
}

async fn get_results_data(
    store: &SqlStore,
    filters: &ResultsFilters,
    registry: &LanguageRegistry,
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
    let use_filters = filters.target.is_some()
        || filters.status.is_some()
        || filters.language.is_some()
        || filters.mutation_types.is_some()
        || filters.line.is_some();

    // Parse mutation types CSV
    let mutation_slugs = parse_csv::<String>(filters.mutation_types.as_deref());

    let mut results = if use_filters {
        store
            .get_outcomes_filtered(
                filters.target.clone(),
                normalize_status(filters.status.clone()),
                filters.language.clone(),
                mutation_slugs,
                filters.line,
                registry,
            )
            .await
            .map_err(|e| -> crate::types::AppError { e.into() })?
    } else {
        // Legacy path: no filters, use old logic with target filtering or config
        let filtered_targets =
            Target::filter_by_path_or_config(store, filters.target.clone()).await?;
        let mut results_vec = Vec::new();

        for target in filtered_targets {
            let mut mutants = store.get_mutants(target.id).await?;
            mutants.sort_by_key(|m| m.byte_offset);

            for mutant in mutants {
                if let Some(outcome) = store.get_outcome(mutant.id).await? {
                    // Filter based on flags
                    if filters.all || filters.verbose || outcome.status == Status::Uncaught {
                        results_vec.push((mutant, target.clone(), outcome));
                    }
                }
            }
        }

        results_vec
    };

    // Apply severity filter if provided (application-layer filtering)
    if let Some(severities) = parse_csv::<MutationSeverity>(filters.severity.as_deref()) {
        results.retain(|(mutant, target, _outcome)| {
            if let Some(mutation) = registry.get_mutation(&target.language, &mutant.mutation_slug) {
                severities.contains(&mutation.severity)
            } else {
                false // Filter out unknown mutations
            }
        });
    }

    Ok(results)
}

async fn print_table_format(
    data: &[(Mutant, Target, Outcome)],
    filters: &ResultsFilters,
    registry: &LanguageRegistry,
    priorities: &HashMap<i64, Annotation>,
) -> AppResult<()> {
    // If mutant_id is provided, special handling
    if let Some(id) = filters.id {
        if data.is_empty() {
            info!("No outcome found for mutant with ID: {}", id);
        } else {
            let (mutant, target, outcome) = &data[0];
            info!("Target: {}", target.display());
            print_outcome(
                mutant,
                target,
                outcome,
                filters.verbose,
                priorities.get(&mutant.id),
            );
        }
        return Ok(());
    }

    // Use filtered query if any filters are provided
    let use_filters = filters.target.is_some()
        || filters.status.is_some()
        || filters.language.is_some()
        || filters.mutation_types.is_some()
        || filters.line.is_some();

    if use_filters {
        if data.is_empty() {
            info!("No outcomes found matching the filters");
            return Ok(());
        }

        // Group by target path for display
        // Note: Data is already sorted by path from database query,
        // BTreeMap maintains this order since we insert in sorted order
        let mut by_target: BTreeMap<String, Vec<&(Mutant, Target, Outcome)>> = BTreeMap::new();
        for entry in data {
            let path_key = entry.1.path.to_string_lossy().to_string();
            by_target.entry(path_key).or_default().push(entry);
        }

        // Display grouped results
        for entries in by_target.values() {
            if entries.is_empty() {
                continue;
            }
            let target = &entries[0].1;
            info!("Target: {}", target.display());

            for (mutant, target, outcome) in entries {
                print_outcome(
                    mutant,
                    target,
                    outcome,
                    filters.verbose,
                    priorities.get(&mutant.id),
                );
            }
            info!(""); // Empty line between targets
        }

        return Ok(());
    }

    // Legacy path: display with per-target statistics
    // Use the already-filtered data instead of fetching from database again
    if data.is_empty() {
        info!("No outcomes found");
        return Ok(());
    }

    // Group data by target
    let mut by_target: BTreeMap<String, Vec<&(Mutant, Target, Outcome)>> = BTreeMap::new();
    for entry in data {
        let path_key = entry.1.path.to_string_lossy().to_string();
        by_target.entry(path_key).or_default().push(entry);
    }

    for entries in by_target.values() {
        if entries.is_empty() {
            continue;
        }

        let target = &entries[0].1;
        info!("Target: {}", target.display());

        let mut has_outcomes = false;
        let mut overall = OutcomeCounter::new();
        let mut high = OutcomeCounter::new();
        let mut medium = OutcomeCounter::new();
        let mut low = OutcomeCounter::new();

        for (mutant, target, outcome) in entries {
            let status = &outcome.status;
            overall.record(status);

            // Get severity using registry
            let severity = if let Some(mutation) =
                registry.get_mutation(&target.language, &mutant.mutation_slug)
            {
                mutation.severity.clone()
            } else {
                MutationSeverity::Low
            };

            match severity {
                MutationSeverity::High => high.record(status),
                MutationSeverity::Medium => medium.record(status),
                MutationSeverity::Low => low.record(status),
            };

            has_outcomes = true;
            print_outcome(
                mutant,
                target,
                outcome,
                filters.verbose,
                priorities.get(&mutant.id),
            );
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

#[cfg(test)]
mod priority_tests {
    use super::*;
    use crate::types::Hash;
    use chrono::Utc;
    use std::path::PathBuf;

    #[test]
    fn json_adds_optional_test_goal_priority_without_changing_existing_fields() {
        let source = "fn value() -> bool { true }";
        let target = Target {
            id: 4,
            path: PathBuf::from("example.rs"),
            file_hash: Hash::digest(source.into()),
            text: source.into(),
            language: "rust".parse().unwrap(),
        };
        let mutant = Mutant {
            id: 5,
            target_id: 4,
            mutation_slug: "BL".into(),
            byte_offset: 21,
            line_offset: 0,
            old_text: "true".into(),
            new_text: "false".into(),
        };
        let outcome = Outcome {
            mutant_id: 5,
            status: Status::Uncaught,
            output: String::new(),
            time: Utc::now(),
            duration_ms: 10,
        };
        let plain = serde_json::to_value(JsonResult {
            mutant: mutant.clone(),
            target: target.clone(),
            outcome: Outcome {
                mutant_id: outcome.mutant_id,
                status: Status::Uncaught,
                output: outcome.output.clone(),
                time: outcome.time,
                duration_ms: outcome.duration_ms,
            },
            test_goal_priority: None,
        })
        .unwrap();
        assert!(plain.get("test_goal_priority").is_none());
        let annotated = serde_json::to_value(JsonResult {
            mutant,
            target,
            outcome,
            test_goal_priority: Some(Annotation {
                score: 2.5,
                confidence: 0.4,
                category: Some("unclear".into()),
            }),
        })
        .unwrap();
        assert_eq!(plain["mutant"], annotated["mutant"]);
        assert_eq!(plain["target"], annotated["target"]);
        assert_eq!(plain["outcome"], annotated["outcome"]);
        assert_eq!(annotated["test_goal_priority"]["score"], 2.5);
        assert_eq!(annotated["test_goal_priority"]["category"], "unclear");
    }
}
