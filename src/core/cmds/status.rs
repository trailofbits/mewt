use log::info;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::core::cli::StatusArgs;
use crate::types::{AppResult, MutationSeverity};

#[derive(Debug, Serialize)]
struct TargetStats {
    path: String,
    total_mutants: usize,
    tested: usize,
    untested: usize,
    caught: usize,
    uncaught: usize,
    timeout: usize,
    skipped: usize,
    high_catch_rate: Option<f64>,
    medium_catch_rate: Option<f64>,
    low_catch_rate: Option<f64>,
}

#[derive(Debug, Serialize)]
struct CampaignStats {
    total_targets: usize,
    total_mutants: usize,
    tested: usize,
    untested: usize,
    caught: usize,
    uncaught: usize,
    timeout: usize,
    skipped: usize,
    high_catch_rate: Option<f64>,
    medium_catch_rate: Option<f64>,
    low_catch_rate: Option<f64>,
    progress_percent: f64,
}

#[derive(Debug, Serialize)]
struct StatusReport {
    targets: Vec<TargetStats>,
    campaign: CampaignStats,
}

pub async fn execute_status(
    args: StatusArgs,
    store: SqlStore,
    registry: Arc<LanguageRegistry>,
) -> AppResult<()> {
    let report = generate_status_report(&store, &registry).await?;

    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&report)?;
            println!("{}", json);
        }
        _ => {
            print_table_format(&report);
        }
    }

    Ok(())
}

async fn generate_status_report(
    store: &SqlStore,
    registry: &LanguageRegistry,
) -> AppResult<StatusReport> {
    let targets = store.get_all_targets().await?;

    if targets.is_empty() {
        return Ok(StatusReport {
            targets: vec![],
            campaign: CampaignStats {
                total_targets: 0,
                total_mutants: 0,
                tested: 0,
                untested: 0,
                caught: 0,
                uncaught: 0,
                timeout: 0,
                skipped: 0,
                high_catch_rate: None,
                medium_catch_rate: None,
                low_catch_rate: None,
                progress_percent: 0.0,
            },
        });
    }

    let mut target_stats = Vec::new();
    let mut campaign_totals = CampaignStats {
        total_targets: targets.len(),
        total_mutants: 0,
        tested: 0,
        untested: 0,
        caught: 0,
        uncaught: 0,
        timeout: 0,
        skipped: 0,
        high_catch_rate: None,
        medium_catch_rate: None,
        low_catch_rate: None,
        progress_percent: 0.0,
    };

    for target in targets {
        let stats = store.get_target_stats(target.id).await?;
        let language_engine = registry.get_engine(&target.language);

        // Compute severity-based catch rates from slug-based stats
        let (high_rate, medium_rate, low_rate) =
            compute_severity_catch_rates(&stats.severity_stats, language_engine);

        campaign_totals.total_mutants += stats.total_mutants;
        campaign_totals.tested += stats.tested;
        campaign_totals.untested += stats.untested;
        campaign_totals.caught += stats.caught;
        campaign_totals.uncaught += stats.uncaught;
        campaign_totals.timeout += stats.timeout;
        campaign_totals.skipped += stats.skipped;

        target_stats.push(TargetStats {
            path: target.path.to_string_lossy().to_string(),
            total_mutants: stats.total_mutants,
            tested: stats.tested,
            untested: stats.untested,
            caught: stats.caught,
            uncaught: stats.uncaught,
            timeout: stats.timeout,
            skipped: stats.skipped,
            high_catch_rate: high_rate,
            medium_catch_rate: medium_rate,
            low_catch_rate: low_rate,
        });
    }

    // Calculate campaign-wide catch rates by severity
    let campaign_severity_stats = store.get_campaign_severity_stats().await?;

    // Get all language engines to map slugs to severities
    let all_languages = registry.all_languages();
    let mut all_engines = Vec::new();
    for lang in all_languages {
        if let Some(engine) = registry.get_engine(lang) {
            all_engines.push(engine);
        }
    }

    let (high_rate, medium_rate, low_rate) = if all_engines.is_empty() {
        (None, None, None)
    } else {
        // For campaign-wide stats, try each engine to resolve slugs
        let mut severity_stats: HashMap<MutationSeverity, (usize, usize)> = HashMap::new();

        for (slug, (eligible, caught)) in &campaign_severity_stats.severity_stats {
            // Try to find the severity from any engine
            let mut found_severity = None;
            for engine in &all_engines {
                if let Some(severity) = engine.get_severity_by_slug(slug) {
                    found_severity = Some(severity);
                    break;
                }
            }

            if let Some(severity) = found_severity {
                let entry = severity_stats.entry(severity).or_insert((0, 0));
                entry.0 += eligible;
                entry.1 += caught;
            }
        }

        let high_rate = calculate_catch_rate(&severity_stats, &MutationSeverity::High);
        let medium_rate = calculate_catch_rate(&severity_stats, &MutationSeverity::Medium);
        let low_rate = calculate_catch_rate(&severity_stats, &MutationSeverity::Low);

        (high_rate, medium_rate, low_rate)
    };

    campaign_totals.high_catch_rate = high_rate;
    campaign_totals.medium_catch_rate = medium_rate;
    campaign_totals.low_catch_rate = low_rate;

    // Calculate progress (exclude skipped mutants from denominator)
    let testable_mutants = campaign_totals.total_mutants - campaign_totals.skipped;
    if testable_mutants > 0 {
        campaign_totals.progress_percent =
            (campaign_totals.tested as f64 / testable_mutants as f64) * 100.0;
    }

    Ok(StatusReport {
        targets: target_stats,
        campaign: campaign_totals,
    })
}

fn compute_severity_catch_rates(
    slug_stats: &HashMap<String, (usize, usize)>,
    language_engine: Option<&dyn crate::LanguageEngine>,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let Some(engine) = language_engine else {
        return (None, None, None);
    };

    // Aggregate slug-level stats into severity-level stats
    let mut severity_stats: HashMap<MutationSeverity, (usize, usize)> = HashMap::new();

    for (slug, (eligible, caught)) in slug_stats {
        if let Some(severity) = engine.get_severity_by_slug(slug) {
            let entry = severity_stats.entry(severity).or_insert((0, 0));
            entry.0 += eligible;
            entry.1 += caught;
        }
    }

    let high_rate = calculate_catch_rate(&severity_stats, &MutationSeverity::High);
    let medium_rate = calculate_catch_rate(&severity_stats, &MutationSeverity::Medium);
    let low_rate = calculate_catch_rate(&severity_stats, &MutationSeverity::Low);

    (high_rate, medium_rate, low_rate)
}

fn calculate_catch_rate(
    severity_stats: &HashMap<MutationSeverity, (usize, usize)>,
    severity: &MutationSeverity,
) -> Option<f64> {
    severity_stats.get(severity).and_then(|(eligible, caught)| {
        if *eligible > 0 {
            Some((*caught as f64 / *eligible as f64) * 100.0)
        } else {
            None
        }
    })
}

fn print_table_format(report: &StatusReport) {
    info!("Campaign Status Report");
    info!("");
    info!("Per-Target Breakdown:");
    info!("=====================");

    if report.targets.is_empty() {
        info!("No targets found. Use the 'run' command with a target to start a campaign.");
        return;
    }

    for target in &report.targets {
        info!("");
        info!("Target: {}", target.path);
        info!(
            "  Mutants: {} total, {} tested, {} untested",
            target.total_mutants, target.tested, target.untested
        );
        info!(
            "  Outcomes: {} caught, {} uncaught, {} timeout, {} skipped",
            target.caught, target.uncaught, target.timeout, target.skipped
        );

        // Catch rates by severity
        let high_rate = format_rate(target.high_catch_rate);
        let medium_rate = format_rate(target.medium_catch_rate);
        let low_rate = format_rate(target.low_catch_rate);
        info!(
            "  Catch rates: High: {}, Medium: {}, Low: {}",
            high_rate, medium_rate, low_rate
        );
    }

    info!("");
    info!("Campaign-Wide Summary:");
    info!("======================");
    let c = &report.campaign;
    info!("Targets: {}", c.total_targets);
    info!(
        "Mutants: {} total, {} tested ({:.1}% complete), {} untested",
        c.total_mutants, c.tested, c.progress_percent, c.untested
    );
    info!(
        "Outcomes: {} caught, {} uncaught, {} timeout, {} skipped",
        c.caught, c.uncaught, c.timeout, c.skipped
    );

    let high_rate = format_rate(c.high_catch_rate);
    let medium_rate = format_rate(c.medium_catch_rate);
    let low_rate = format_rate(c.low_catch_rate);
    info!(
        "Catch rates by severity: High: {}, Medium: {}, Low: {}",
        high_rate, medium_rate, low_rate
    );
}

fn format_rate(rate: Option<f64>) -> String {
    match rate {
        Some(r) => format!("{:.1}%", r),
        None => "N/A".to_string(),
    }
}
