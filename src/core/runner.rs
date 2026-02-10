use chrono::Utc;
use log::{debug, error, info, warn};
use std::io;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::core::logging::{end_progress_bar, new_progress_bar};
use indicatif::{HumanDuration, ProgressBar};

use crate::LanguageRegistry;
use crate::SqlStore;
use crate::types::{CampaignSummary, Mutant, MutationSeverity, Outcome, Status, Target};

pub struct TestRunner {
    test_cmd: String,
    timeout: Option<Duration>,
    comprehensive: bool,
    verbose: bool,
    running: Arc<AtomicBool>,
    store: SqlStore,
    registry: Arc<LanguageRegistry>,
    // Track if we've applied mutations that need to be cleaned up
    has_active_mutation: bool,
    // Hold the current target for cleanup
    current_target: Option<Target>,
    // Track uncaught high severity mutant lines (blocks medium and low severity tests)
    uncaught_high_sev_lines: std::collections::HashSet<u32>,
    // Track uncaught medium severity mutant lines (only blocks low severity tests)
    uncaught_med_sev_lines: std::collections::HashSet<u32>,
    // Campaign-wide progress bar to track all mutants across all targets
    campaign_bar: Option<ProgressBar>,
}

impl TestRunner {
    fn new(
        test_cmd: String,
        timeout_secs: Option<u32>,
        comprehensive: bool,
        verbose: bool,
        running: Arc<AtomicBool>,
        store: SqlStore,
        registry: Arc<LanguageRegistry>,
    ) -> Self {
        Self {
            test_cmd,
            timeout: timeout_secs.map(|secs| Duration::from_secs(secs as u64)),
            comprehensive,
            verbose,
            running,
            store,
            registry,
            has_active_mutation: false,
            current_target: None,
            uncaught_high_sev_lines: std::collections::HashSet::new(),
            uncaught_med_sev_lines: std::collections::HashSet::new(),
            campaign_bar: None,
        }
    }

    /// Creates a TestRunner, runs baseline tests, and configures timeout settings
    /// Returns a properly configured TestRunner ready for mutation testing
    pub async fn new_with_baseline(
        test_cmd: String,
        user_timeout: Option<u32>,
        running: Arc<AtomicBool>,
        store: SqlStore,
        comprehensive: bool,
        verbose: bool,
        registry: Arc<LanguageRegistry>,
    ) -> Result<Self, io::Error> {
        // Create initial runner for baseline tests (no timeout)
        let mut runner = Self::new(
            test_cmd.clone(),
            None,
            comprehensive,
            verbose,
            Arc::clone(&running),
            store.clone(),
            Arc::clone(&registry),
        );

        // Run baseline tests
        let baseline_duration_ms = runner.run_baseline_test().await?;
        let baseline_duration_secs = baseline_duration_ms.div_ceil(1000);
        let recommended_timeout = baseline_duration_secs * 2;

        // Determine the actual timeout to use
        let actual_timeout = match user_timeout {
            Some(user_timeout) => {
                if user_timeout < baseline_duration_secs {
                    error!(
                        "Specified timeout ({user_timeout} seconds) is less than baseline test runtime ({baseline_duration_secs} seconds)"
                    );
                    error!(
                        "This will cause all mutation tests to time out. Please specify a longer timeout."
                    );
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Timeout too short",
                    ));
                } else if user_timeout < recommended_timeout {
                    warn!(
                        "Specified timeout ({user_timeout} seconds) is less than recommended 2x baseline runtime ({recommended_timeout} seconds)"
                    );
                    warn!("Some legitimate tests may time out. Consider increasing the timeout.");
                    user_timeout
                } else {
                    user_timeout
                }
            }
            None => {
                info!(
                    "No timeout specified, using 2x baseline test runtime: {recommended_timeout} seconds"
                );
                recommended_timeout
            }
        };

        // Create a new TestRunner with the determined timeout for mutation tests
        Ok(Self::new(
            test_cmd,
            Some(actual_timeout),
            comprehensive,
            verbose,
            running,
            store,
            registry,
        ))
    }

    pub async fn run_baseline_test(&mut self) -> Result<u32, io::Error> {
        if self.timeout.is_some() {
            return Err(io::Error::other(
                "Baseline tests should run with no timeout",
            ));
        }
        info!("Running baseline test to ensure tests pass before applying mutations...");

        let start = Instant::now();
        let (status, output) = self.run_and_wait()?;
        let duration_ms = start.elapsed().as_millis() as u32;

        if status != Status::Uncaught {
            error!("Baseline test failed! Fix your tests before running mutation testing.");
            if !self.verbose {
                error!("Test output:\n{output}");
            }
            return Err(io::Error::other("Baseline test failed"));
        }

        info!("Baseline test passed successfully!");
        Ok(duration_ms)
    }

    pub async fn run_mutation_campaign(
        &mut self,
        targets: Vec<Target>,
        filter_slugs: Option<String>,
    ) -> io::Result<CampaignSummary> {
        // Parse mutation slugs if provided
        let allowed_slugs: Option<Vec<String>> = filter_slugs.map(|s| {
            let slugs: Vec<String> = s.split(',').map(|s| s.trim().to_string()).collect();
            info!("Filtering mutations to test by slugs: {}", slugs.join(", "));
            slugs
        });

        // Count total mutants to be tested across all targets for time estimation
        let mut total_untested_mutants = 0;
        let mut campaign_untested_count = 0;
        let mut campaign_retest_count = 0;

        for target in &targets {
            match self.store.get_mutant_test_counts(target.id).await {
                Ok((untested, retest)) => {
                    campaign_untested_count += untested;
                    campaign_retest_count += retest;
                    total_untested_mutants += untested + retest;
                }
                Err(e) => {
                    error!("Failed to get test counts for target {}: {}", target.id, e);
                }
            }
        }

        // Calculate estimated time for whole campaign
        let timeout_secs = self.timeout.map(|t| t.as_secs()).unwrap_or(0);
        let estimated_total_duration =
            Duration::from_secs(timeout_secs * total_untested_mutants as u64);

        if campaign_untested_count > 0 && campaign_retest_count > 0 {
            info!(
                "Starting mutation campaign with {} targets ({} untested mutants, {} to be retested)",
                targets.len(),
                campaign_untested_count,
                campaign_retest_count
            );
        } else if campaign_untested_count > 0 {
            info!(
                "Starting mutation campaign with {} targets ({} untested mutants)",
                targets.len(),
                campaign_untested_count
            );
        } else if campaign_retest_count > 0 {
            info!(
                "Starting mutation campaign with {} targets ({} mutants to be retested)",
                targets.len(),
                campaign_retest_count
            );
        } else {
            info!(
                "Starting mutation campaign with {} targets (no mutants to test)",
                targets.len()
            );
        }
        info!(
            "Estimated maximum completion time: {}",
            HumanDuration(estimated_total_duration)
        );

        // Create a single campaign-wide progress bar that tracks all mutants
        if total_untested_mutants > 1 {
            self.campaign_bar = Some(new_progress_bar(
                total_untested_mutants as u64,
                "Preparing mutation campaign...",
            ));
        }

        let campaign_start = Instant::now();

        // Instead of using a guard, we'll use a try-finally pattern with manual cleanup
        let result = self
            .run_mutation_campaign_inner(targets, allowed_slugs)
            .await;

        // Always do cleanup if needed, regardless of whether an error occurred
        if self.has_active_mutation {
            let _ = self.cleanup();
        }

        // Finish and clear the campaign progress bar if it exists
        if let Some(bar) = &self.campaign_bar {
            end_progress_bar(bar);
        }
        self.campaign_bar = None;

        // Calculate actual campaign duration
        let campaign_duration = campaign_start.elapsed();
        info!(
            "Mutation campaign completed in {}",
            HumanDuration(campaign_duration)
        );

        // Check the result
        result?;

        // Get campaign summary from database
        let summary = self
            .store
            .get_campaign_summary()
            .await
            .map_err(|e| io::Error::other(format!("Failed to get campaign summary: {}", e)))?;

        // Print summary
        info!("");
        info!("Campaign Summary:");
        info!("  Tested:   {} mutants", summary.tested);
        info!("  Caught:   {} mutants", summary.caught);
        info!("  Uncaught: {} mutants", summary.uncaught);
        info!("  Skipped:  {} mutants", summary.skipped);

        if summary.tested == 0 {
            info!("No mutants were tested");
        } else {
            info!("Mutation testing campaign completed successfully");
        }

        Ok(summary)
    }

    async fn run_mutation_campaign_inner(
        &mut self,
        targets: Vec<Target>,
        allowed_slugs: Option<Vec<String>>,
    ) -> io::Result<()> {
        let total_targets = targets.len();
        for (idx, target) in targets.into_iter().enumerate() {
            if !self.running.load(Ordering::SeqCst) {
                info!("Mutation campaign interrupted, stopping...");
                break;
            }

            // Update the campaign bar message to show current target position and path
            if let Some(bar) = &self.campaign_bar {
                bar.set_message(format!(
                    "target {}/{} {}",
                    idx + 1,
                    total_targets,
                    target.display()
                ));
            }

            self.run_mutations_for_target(target, allowed_slugs.clone())
                .await?;
        }

        Ok(())
    }

    async fn run_mutations_for_target(
        &mut self,
        target: Target,
        allowed_slugs: Option<Vec<String>>,
    ) -> io::Result<()> {
        info!("");
        info!("Processing target: {}", target.display());
        self.current_target = Some(target.clone());
        // Clear any tracked uncaught lines from previous targets
        self.uncaught_high_sev_lines.clear();
        self.uncaught_med_sev_lines.clear();

        // Get all mutations for this target
        let mut mutants = match self.store.get_mutants(target.id).await {
            Ok(mutants) => mutants,
            Err(e) => {
                error!("Failed to get mutants for target {}: {}", target.id, e);
                return Ok(());
            }
        };

        let language = &target.language;

        // Sort mutants by severity (High, Medium, Low)
        mutants.sort_by(|a, b| {
            let a_sev = self
                .registry
                .get_engine(language)
                .unwrap()
                .get_severity_by_slug(&a.mutation_slug)
                .map(|s| s.to_numeric())
                .unwrap_or(2); // Default to Low severity if not found
            let b_sev = self
                .registry
                .get_engine(language)
                .unwrap()
                .get_severity_by_slug(&b.mutation_slug)
                .map(|s| s.to_numeric())
                .unwrap_or(2); // Default to Low severity if not found
            a_sev.cmp(&b_sev)
        });

        let mut count = 1;
        let mut skipped = 0;

        // Get counts of untested vs retest mutants for this target
        let (untested_count, retest_count) =
            match self.store.get_mutant_test_counts(target.id).await {
                Ok(counts) => counts,
                Err(e) => {
                    error!("Failed to get test counts for target {}: {}", target.id, e);
                    (0, 0)
                }
            };

        let total_untested = untested_count + retest_count;

        // Estimate time for this target
        let timeout_secs = self.timeout.map(|t| t.as_secs()).unwrap_or(0);
        let estimated_target_duration = Duration::from_secs(timeout_secs * total_untested as u64);

        if untested_count > 0 && retest_count > 0 {
            info!(
                "Found {} mutants for this target ({} untested, {} to be retested), maximum runtime: {}",
                mutants.len(),
                untested_count,
                retest_count,
                HumanDuration(estimated_target_duration),
            );
        } else if untested_count > 0 {
            info!(
                "Found {} mutants for this target ({} untested), maximum runtime: {}",
                mutants.len(),
                untested_count,
                HumanDuration(estimated_target_duration),
            );
        } else if retest_count > 0 {
            info!(
                "Found {} mutants for this target ({} to be retested), maximum runtime: {}",
                mutants.len(),
                retest_count,
                HumanDuration(estimated_target_duration),
            );
        } else {
            info!(
                "Found {} mutants for this target (all have been tested), maximum runtime: {}",
                mutants.len(),
                HumanDuration(estimated_target_duration),
            );
        }

        let target_start = Instant::now();
        let mut target_duration_ms = 0;

        // Keep track of invalid slugs to warn about them only once
        let mut warned_invalid_slugs = Vec::new();

        for mutant in mutants {
            if !self.running.load(Ordering::SeqCst) {
                info!("Mutation testing interrupted, stopping...");
                break;
            }

            // Skip if this mutation already has an outcome, unless it's a Timeout
            if let Ok(Some(outcome)) = self.store.get_outcome(mutant.id).await {
                if outcome.status != Status::Timeout {
                    debug!(
                        "Mutation {} already has a valid outcome, skipping",
                        mutant.id
                    );
                    continue;
                } else {
                    debug!("Mutation {} has timeout outcome, retesting", mutant.id);
                }
            }

            // Skip less severe mutations if more severe ones on the same line were uncaught
            // and comprehensive mode is not enabled
            if !self.comprehensive {
                let severity = self
                    .registry
                    .get_engine(language)
                    .unwrap()
                    .get_severity_by_slug(&mutant.mutation_slug)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("Mutation slug not found: {}", mutant.mutation_slug),
                        )
                    })?
                    .to_numeric();
                let (line_start, line_end) = mutant.get_lines();

                // Check if we should skip based on severity
                let should_skip = match severity {
                    // Medium severity - skip if overlaps with high severity uncaught lines
                    1 => (line_start..=line_end)
                        .any(|line| self.uncaught_high_sev_lines.contains(&line)),
                    // Low severity - skip if overlaps with high OR medium severity uncaught lines
                    2 => (line_start..=line_end).any(|line| {
                        self.uncaught_high_sev_lines.contains(&line)
                            || self.uncaught_med_sev_lines.contains(&line)
                    }),
                    // High severity - never skip
                    _ => false,
                };

                if should_skip {
                    info!(
                        "Skipping {} severity mutation on line {} (higher severity mutations were uncaught)",
                        MutationSeverity::from_numeric(severity),
                        line_start
                    );

                    // Create a skipped outcome
                    let outcome = Outcome {
                        mutant_id: mutant.id,
                        status: Status::Skipped,
                        output: String::from(
                            "Skipped due to uncaught higher severity mutation on the same line",
                        ),
                        time: Utc::now(),
                        duration_ms: 0,
                    };

                    if let Err(e) = self.store.add_outcome(outcome).await {
                        error!(
                            "Failed to store skipped outcome for mutant {}: {}",
                            mutant.id, e
                        );
                    }

                    skipped += 1;
                    if let Some(bar) = &self.campaign_bar {
                        bar.inc(1);
                    }
                    continue;
                }
            }

            // If we're filtering by mutation type, check if this one matches
            if let Some(slugs) = &allowed_slugs {
                if !slugs.is_empty() {
                    // Check if the mutant's slug is in our allowed list
                    if !slugs.iter().any(|s| s == &mutant.mutation_slug) {
                        // Skip this mutant as its slug is not in our allowed list
                        skipped += 1;
                        if let Some(bar) = &self.campaign_bar {
                            bar.inc(1);
                        }
                        continue;
                    } else if !warned_invalid_slugs.contains(&mutant.mutation_slug) {
                        // For invalid slugs, we'll warn the user but only once per slug
                        let valid_slugs =
                            self.registry.get_engine(language).unwrap().get_all_slugs();

                        if !valid_slugs.contains(&mutant.mutation_slug) {
                            warn!("Unknown mutation slug: {}", mutant.mutation_slug);
                            warned_invalid_slugs.push(mutant.mutation_slug.clone());
                        }
                    }
                }
            }

            info!(
                "  Testing mutation {}/{}: {}",
                count,
                total_untested - skipped,
                mutant.display(&target)
            );
            self.test_mutant(target.clone(), mutant, &mut target_duration_ms)
                .await?;
            count += 1;
            if let Some(bar) = &self.campaign_bar {
                bar.inc(1);
            }
        }

        if skipped > 0 {
            info!("Skipped {skipped} mutations");
        }

        // Calculate actual target duration
        let target_elapsed = target_start.elapsed();
        info!(
            "Finished testing target in {}",
            HumanDuration(target_elapsed),
        );
        info!("");

        // Clear current target after processing
        self.current_target = None;

        Ok(())
    }

    pub async fn test_mutant(
        &mut self,
        target: Target,
        mutant: Mutant,
        target_duration_ms: &mut u32,
    ) -> io::Result<()> {
        // Apply the mutation
        let mutated_target = target.mutate(&mutant)?;
        self.has_active_mutation = true;
        std::fs::write(&target.path, mutated_target)?;

        // Run & time the test
        let start_time = Instant::now();

        let result = self.run_and_wait();

        // Handle interruption specially
        if let Err(e) = &result {
            if e.kind() == io::ErrorKind::Interrupted {
                // Just restore the file and exit without creating an outcome
                target.restore()?;
                self.has_active_mutation = false;
                return Ok(());
            }
        }

        let (status, output) = result?;

        let duration = start_time.elapsed();
        let duration_ms = duration.as_millis() as u32;

        // If this was uncaught and it's a high or medium severity mutant,
        // track the affected lines so we can skip lower severity mutants on those lines
        if status == Status::Uncaught {
            let language = &target.language;
            let severity = self
                .registry
                .get_engine(language)
                .unwrap()
                .get_severity_by_slug(&mutant.mutation_slug)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Mutation slug not found: {}", mutant.mutation_slug),
                    )
                })?
                .to_numeric();
            // Only track High (0) and Medium (1) severity uncaught mutants
            if severity == 0 || severity == 1 {
                let (line_start, line_end) = mutant.get_lines();
                debug!(
                    "Tracking uncaught {} severity mutant on line {}",
                    MutationSeverity::from_numeric(severity),
                    line_start
                );
                for line in line_start..=line_end {
                    // Track in the appropriate collection based on severity
                    if severity == 0 {
                        self.uncaught_high_sev_lines.insert(line);
                    } else if severity == 1 {
                        self.uncaught_med_sev_lines.insert(line);
                    }
                }
            }
        }

        // Create outcome
        let outcome = Outcome {
            mutant_id: mutant.id,
            status,
            output,
            time: Utc::now(),
            duration_ms,
        };

        // Add this test's duration to the target's total runtime
        *target_duration_ms += duration_ms;

        // Store outcome
        if let Err(e) = self.store.add_outcome(outcome).await {
            error!("Failed to store outcome for mutant {}: {}", mutant.id, e);
        }

        // Restore original file
        target.restore()?;
        self.has_active_mutation = false;

        Ok(())
    }

    fn run_and_wait(&mut self) -> io::Result<(Status, String)> {
        use std::sync::mpsc;
        use std::thread;

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&self.test_cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let start = Instant::now();
        let mut stdout = "STDOUT:\n".to_string();
        let mut stderr = "STDERR:\n".to_string();
        let verbose = self.verbose;

        // Create channels for stdout and stderr
        let (stdout_tx, stdout_rx) = mpsc::channel();
        let (stderr_tx, stderr_rx) = mpsc::channel();

        // Spawn thread for stdout
        if let Some(stdout) = child.stdout.take() {
            let tx = stdout_tx.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if verbose {
                        info!("{line}");
                    }
                    if tx.send(format!("{line}\n")).is_err() {
                        // Channel receiver dropped, likely due to process termination
                        break;
                    }
                }
            });
        }

        // Spawn thread for stderr
        if let Some(stderr) = child.stderr.take() {
            let tx = stderr_tx.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    if verbose {
                        error!("{line}");
                    }
                    if tx.send(format!("{line}\n")).is_err() {
                        // Channel receiver dropped, likely due to process termination
                        break;
                    }
                }
            });
        }

        loop {
            if let Some(timeout) = self.timeout {
                if start.elapsed() >= timeout {
                    warn!("test timeout reached, killing process");
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok((Status::Timeout, format!("{stdout}\n\n{stderr}")));
                }
            }

            // Check if we should terminate due to ctrl-c
            if !self.running.load(Ordering::SeqCst) {
                warn!("Process interrupted, killing child");
                let _ = child.kill();
                let _ = child.wait();
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Test interrupted",
                ));
            }

            // Collect any new output from channels
            let mut stdout_lines = Vec::new();
            while let Ok(line) = stdout_rx.try_recv() {
                stdout_lines.push(line);
            }
            if !stdout_lines.is_empty() {
                for line in stdout_lines {
                    stdout.push_str(&line);
                }
            }

            let mut stderr_lines = Vec::new();
            while let Ok(line) = stderr_rx.try_recv() {
                stderr_lines.push(line);
            }
            if !stderr_lines.is_empty() {
                for line in stderr_lines {
                    stderr.push_str(&line);
                }
            }

            // Check if the process has exited
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process has completed, collect any remaining output
                    let mut final_stdout_lines = Vec::new();
                    while let Ok(line) = stdout_rx.try_recv() {
                        final_stdout_lines.push(line);
                    }
                    if !final_stdout_lines.is_empty() {
                        for line in final_stdout_lines {
                            stdout.push_str(&line);
                        }
                    }

                    let mut final_stderr_lines = Vec::new();
                    while let Ok(line) = stderr_rx.try_recv() {
                        final_stderr_lines.push(line);
                    }
                    if !final_stderr_lines.is_empty() {
                        for line in final_stderr_lines {
                            stderr.push_str(&line);
                        }
                    }

                    // Map exit status to our Status enum
                    let result_status = if status.success() {
                        Status::Uncaught // Test passed with mutation (bad)
                    } else {
                        Status::TestFail // Test failed with mutation (good)
                    };

                    return Ok((result_status, format!("{stdout}\n\n{stderr}")));
                }
                Ok(None) => {
                    // Process is still running
                    // Sleep a bit to avoid CPU spinning
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn cleanup(&mut self) -> io::Result<()> {
        info!("Running cleanup...");
        // Restore original file if mutation is active
        if self.has_active_mutation {
            if let Some(target) = &self.current_target {
                info!("Restoring original file after interrupted mutation");
                target.restore()?;
                self.has_active_mutation = false;
            }
        }
        Ok(())
    }
}

impl Drop for TestRunner {
    fn drop(&mut self) {
        if self.has_active_mutation {
            info!("TestRunner drop ensuring mutation cleanup");
            if let Err(e) = self.cleanup() {
                error!("Error during TestRunner cleanup: {e}");
            }
        }
    }
}
