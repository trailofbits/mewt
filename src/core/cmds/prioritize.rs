//! CLI orchestration for TypeSafe-backed mutant prioritization.
use std::collections::HashSet;

use log::{info, warn};

use crate::SqlStore;
use crate::core::cli::PrioritizeArgs;
use crate::core::prioritize::{self, Annotation, Purpose};
use crate::core::typesafe::Client;
use crate::types::{AppError, AppResult, Mutant, Target};

pub async fn execute_prioritize(
    store: &SqlStore,
    client: &Client,
    command: PrioritizeArgs,
) -> AppResult<()> {
    let (purpose, options) = match command {
        PrioritizeArgs::Mutants(options) => (Purpose::Pre, options),
        PrioritizeArgs::Results(options) => (Purpose::Post, options),
    };
    let targets = select_targets(store, &options.targets).await?;
    evaluate(store, client, &targets, purpose, options.force).await?;
    Ok(())
}

async fn select_targets(store: &SqlStore, patterns: &[String]) -> AppResult<Vec<Target>> {
    let targets = store.get_all_targets().await?;
    if patterns.is_empty() {
        return Ok(targets);
    }
    let mut matchers = Vec::new();
    for pattern in patterns {
        let path = std::path::PathBuf::from(pattern);
        // A literal saved path takes precedence even if its name contains '['.
        let glob = if targets
            .iter()
            .any(|t| t.path == path || t.path.starts_with(&path))
        {
            None
        } else {
            Some(
                globset::Glob::new(pattern)
                    .map_err(|e| {
                        AppError::Custom(format!("Invalid target pattern {pattern:?}: {e}"))
                    })?
                    .compile_matcher(),
            )
        };
        matchers.push((path, glob));
    }
    let mut selected = HashSet::new();
    for (path, glob) in &matchers {
        let matches: Vec<_> = targets
            .iter()
            .filter(|t| {
                t.path == *path
                    || t.path.starts_with(path)
                    || glob.as_ref().is_some_and(|g| g.is_match(&t.path))
            })
            .map(|t| t.id)
            .collect();
        if matches.is_empty() {
            return Err(AppError::Custom(format!(
                "No saved targets match {:?}; run 'mewt mutate TARGET' first",
                path
            )));
        }
        selected.extend(matches);
    }
    Ok(targets
        .into_iter()
        .filter(|t| selected.contains(&t.id))
        .collect())
}

#[derive(Default, Debug)]
struct Counts {
    pub cached: usize,
    pub evaluated: usize,
    pub failed: usize,
    pub requests: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

struct ScoredMutant {
    id: i64,
    path: String,
    line: u32,
    annotation: Annotation,
}

impl ScoredMutant {
    fn new(target: &Target, mutant: &Mutant, annotation: Annotation) -> Self {
        Self {
            id: mutant.id,
            path: target.display_path(),
            line: mutant.line_offset + 1,
            annotation,
        }
    }
}

fn report(counts: &Counts, purpose: Purpose, scored: &mut [ScoredMutant]) {
    info!(
        "Priority {}: {} evaluated, {} cached, {} failed ({} requests, {} input / {} output tokens)",
        purpose.as_str(),
        counts.evaluated,
        counts.cached,
        counts.failed,
        counts.requests,
        counts.input_tokens,
        counts.output_tokens
    );
    if scored.is_empty() {
        return;
    }
    let mut bands = [0; 4];
    for item in scored.iter() {
        bands[(item.annotation.score.floor() as usize).min(3)] += 1;
    }
    info!(
        "  Available score bands 0–<1: {}, 1–<2: {}, 2–<3: {}, 3–4: {} (heuristic; no recommended threshold)",
        bands[0], bands[1], bands[2], bands[3]
    );
    if purpose == Purpose::Pre {
        scored.sort_by(|a, b| {
            a.annotation
                .score
                .total_cmp(&b.annotation.score)
                .then_with(|| a.id.cmp(&b.id))
        });
        info!("  Lowest-scoring available mutants (not proof of redundancy):");
    } else {
        scored.sort_by(|a, b| {
            b.annotation
                .score
                .total_cmp(&a.annotation.score)
                .then_with(|| a.id.cmp(&b.id))
        });
        info!("  Highest-scoring available test goals (results keep their usual order):");
    }
    for item in scored.iter().take(5) {
        let focus = item
            .annotation
            .category
            .as_deref()
            .map(|s| format!(", test focus: {}", s.replace('_', " ")))
            .unwrap_or_default();
        info!(
            "    #{} {}:{}  {:.2}/4 (confidence {:.2}{focus})",
            item.id, item.path, item.line, item.annotation.score, item.annotation.confidence
        );
    }
}

/// Sequential batches bound in-flight request count to one. A failed chunk
/// leaves every previous row untouched; successful chunks remain resumable.
async fn evaluate(
    store: &SqlStore,
    client: &Client,
    targets: &[Target],
    purpose: Purpose,
    force: bool,
) -> AppResult<Counts> {
    let mut counts = Counts::default();
    let mut scored = Vec::new();
    for target in targets {
        let plan = match prioritize::prepare(store, target, purpose).await {
            Ok(plan) => plan,
            Err(e) => {
                warn!(
                    "Could not prepare priority judgments for {}: {e}",
                    target.display()
                );
                // Eligibility may be unknown if the database read failed.
                counts.failed += store
                    .get_mutants(target.id)
                    .await
                    .map_or(1, |m| m.len().max(1));
                continue;
            }
        };
        for (id, reason) in &plan.unsupported {
            warn!("Priority judgment unavailable for mutant {id}: {reason}");
            counts.failed += 1;
        }
        for batch in &plan.batches {
            let mut pending = Vec::new();
            for mutant in &batch.mutants {
                if !force {
                    match prioritize::cached_annotation(
                        store,
                        target,
                        mutant,
                        purpose,
                        &batch.request,
                    )
                    .await
                    {
                        Ok(Some(annotation)) => {
                            counts.cached += 1;
                            scored.push(ScoredMutant::new(target, mutant, annotation));
                            continue;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            warn!("Priority cache read failed for mutant {}: {e}", mutant.id);
                            counts.failed += 1;
                            continue;
                        }
                    }
                }
                pending.push(mutant);
            }
            if pending.is_empty() {
                continue;
            }
            // The shared state and *all* questions are part of each fingerprint.
            // Re-query the original batch after a partial hit so the cache stays
            // valid; only write entries that needed a refresh.
            counts.requests += 1;
            match client.evaluate(&batch.request).await {
                Ok(result) => {
                    counts.input_tokens += result.usage.input_tokens;
                    counts.output_tokens += result.usage.output_tokens;
                    for mutant in pending {
                        match prioritize::store_annotation(
                            store,
                            target,
                            mutant,
                            purpose,
                            &batch.request,
                            &result,
                        )
                        .await
                        {
                            Ok(Some(annotation)) => {
                                scored.push(ScoredMutant::new(target, mutant, annotation));
                                counts.evaluated += 1;
                            }
                            Ok(None) => {
                                warn!("Invalid priority judgment for mutant {}", mutant.id);
                                counts.failed += 1;
                            }
                            Err(e) => {
                                warn!(
                                    "Could not store priority judgment for mutant {}: {e}",
                                    mutant.id
                                );
                                counts.failed += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    // No request/response body, source, or bearer credential in this message.
                    warn!("Priority request failed ({} judgments): {e}", pending.len());
                    counts.failed += pending.len();
                }
            }
        }
    }
    report(&counts, purpose, &mut scored);
    if counts.failed > 0 {
        return Err(AppError::Custom(format!(
            "Priority {} incomplete: {} evaluated, {} cached, {} failed",
            purpose.as_str(),
            counts.evaluated,
            counts.cached,
            counts.failed
        )));
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::prioritize::{annotations, plan, prepare};
    use crate::types::{Hash, Outcome, Status};
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn saved(path: PathBuf, text: &str) -> Target {
        Target {
            id: 1,
            path,
            file_hash: Hash::digest(text.into()),
            text: text.into(),
            language: "rust".parse().unwrap(),
        }
    }

    fn mutation(target: &Target, id: i64, old: &str, new: &str) -> Mutant {
        let offset = target.text.find(old).unwrap();
        Mutant {
            id,
            target_id: target.id,
            byte_offset: offset as u32,
            line_offset: target.text[..offset]
                .bytes()
                .filter(|b| *b == b'\n')
                .count() as u32,
            old_text: old.into(),
            new_text: new.into(),
            mutation_slug: "AOS".into(),
        }
    }

    // Local HTTP mock: synthesize valid Score/Choice answers from the received
    // request, and verify that only selected evidence and no file path is sent.
    async fn server(status: &str) -> (Client, tokio::task::JoinHandle<Value>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = Client::new("test-key")
            .unwrap()
            .with_base_url(format!("http://{}", listener.local_addr().unwrap()));
        let status = status.to_string();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            let mut buf = [0_u8; 4096];
            loop {
                let size = socket.read(&mut buf).await.unwrap();
                assert!(size > 0);
                raw.extend_from_slice(&buf[..size]);
                if let Some(end) = raw.windows(4).position(|b| b == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&raw[..end]).to_ascii_lowercase();
                    assert!(headers.contains("authorization: bearer test-key"));
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if raw.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let end = raw.windows(4).position(|b| b == b"\r\n\r\n").unwrap();
            let request: Value = serde_json::from_slice(&raw[end + 4..]).unwrap();
            let mut answers = serde_json::Map::new();
            for (id, question) in request["questions"].as_object().unwrap() {
                if question["type"] == "score" {
                    let levels = question["criteria"].as_array().unwrap();
                    let score = if id.starts_with("m1_") { 0 } else { 2 };
                    let probs: BTreeMap<String, f64> = (0..5)
                        .map(|i| (i.to_string(), if i == score { 1.0 } else { 0.0 }))
                        .collect();
                    let legend: BTreeMap<String, Value> = levels
                        .iter()
                        .enumerate()
                        .map(|(i, l)| (i.to_string(), l.clone()))
                        .collect();
                    answers.insert(id.clone(), json!({"type":"score","score":score,"confidence":1.0,"legend":legend,"probabilities":probs}));
                } else {
                    let probabilities: BTreeMap<String, f64> = question["criteria"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .map(|key| (key.clone(), if key == "boundary" { 1.0 } else { 0.0 }))
                        .collect();
                    answers.insert(id.clone(), json!({"type":"choice","choice":"boundary","confidence":1.0,"probabilities":probabilities}));
                }
            }
            let body = serde_json::to_string(&json!({"model":"jev-test-v1","usage":{"input_tokens":42,"output_tokens":5},"answers":answers})).unwrap();
            let reply = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(reply.as_bytes()).await.unwrap();
            request
        });
        (client, handle)
    }

    #[tokio::test]
    async fn selects_only_saved_paths_with_or_without_files() {
        let store = SqlStore::new("sqlite::memory:".into()).await.unwrap();
        for name in ["src/a.rs", "src/b.rs", "other/[a.rs"] {
            store
                .add_target(saved(PathBuf::from(name), name))
                .await
                .unwrap();
        }
        assert_eq!(select_targets(&store, &[]).await.unwrap().len(), 3);
        assert_eq!(
            select_targets(&store, &["src".into()]).await.unwrap().len(),
            2
        );
        assert_eq!(
            select_targets(&store, &["src/*.rs".into()])
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            select_targets(&store, &["src/*.rs".into(), "src/a.rs".into()])
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            select_targets(&store, &["other/[a.rs".into()])
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(
            select_targets(&store, &["nonexistent".into()])
                .await
                .is_err()
        );
        assert!(select_targets(&store, &["[bad".into()]).await.is_err());
    }

    #[tokio::test]
    async fn cache_force_failure_and_current_post_evidence() {
        let tmp = tempdir().unwrap();
        let store = SqlStore::new("sqlite::memory:".into()).await.unwrap();
        let mut target = saved(tmp.path().join("f.rs"), "fn f() { 1 + 2; 3 + 4; }\n");
        target.id = store.add_target(target.clone()).await.unwrap();
        for (old, new) in [("1 + 2", "1 - 2"), ("3 + 4", "3 - 4")] {
            let mut m = mutation(&target, 0, old, new);
            m.id = store.add_mutant(m.clone()).await.unwrap().unwrap();
        }
        let planned = prepare(&store, &target, Purpose::Pre).await.unwrap();
        let (client, handle) = server("200 OK").await;
        let counts = evaluate(
            &store,
            &client,
            std::slice::from_ref(&target),
            Purpose::Pre,
            false,
        )
        .await
        .unwrap();
        assert_eq!(counts.evaluated, 2);
        let req = handle.await.unwrap();
        assert_eq!(req["state"]["purpose"], "pre");
        assert!(req.to_string().contains("1 + 2"));
        assert!(!req.to_string().contains("f.rs"));
        let before = annotations(&store, &target, &planned).await.unwrap();
        assert_eq!(before.len(), 2);
        assert_eq!(before[&1].score, 0.0);
        let cached = evaluate(
            &store,
            &client,
            std::slice::from_ref(&target),
            Purpose::Pre,
            false,
        )
        .await
        .unwrap();
        assert_eq!(cached.cached, 2); // No HTTP request made.

        let (bad, handle) = server("401 Unauthorized").await;
        assert!(
            evaluate(
                &store,
                &bad,
                std::slice::from_ref(&target),
                Purpose::Pre,
                true
            )
            .await
            .is_err()
        );
        handle.await.unwrap();
        assert_eq!(
            annotations(&store, &target, &planned).await.unwrap().len(),
            2
        );
        let original = store.get_priority(1, "pre").await.unwrap().unwrap();
        store
            .put_priority(1, "pre", &original.input_hash, &original.model, "not JSON")
            .await
            .unwrap();
        assert!(
            !annotations(&store, &target, &planned)
                .await
                .unwrap()
                .contains_key(&1)
        );
        let mut corrupt_score: Value = serde_json::from_str(&original.payload_json).unwrap();
        corrupt_score["answers"]["m1_signal"]["score"] = json!(4.0);
        let corrupt_score = corrupt_score.to_string();
        store
            .put_priority(
                1,
                "pre",
                &original.input_hash,
                &original.model,
                &corrupt_score,
            )
            .await
            .unwrap();
        assert!(
            !annotations(&store, &target, &planned)
                .await
                .unwrap()
                .contains_key(&1)
        );
        let (repair, handle) = server("200 OK").await;
        let refresh = evaluate(
            &store,
            &repair,
            std::slice::from_ref(&target),
            Purpose::Pre,
            false,
        )
        .await
        .unwrap();
        assert_eq!(refresh.evaluated, 1);
        assert_eq!(refresh.cached, 1);
        assert_eq!(refresh.requests, 1); // The original batch is re-queried after a partial hit.
        handle.await.unwrap();
        assert_eq!(
            annotations(&store, &target, &planned).await.unwrap().len(),
            2
        );
        let current = store.get_priority(1, "pre").await.unwrap().unwrap();
        let mut previous: Value = serde_json::from_str(&current.payload_json).unwrap();
        previous["version"] = json!(1);
        store
            .put_priority(
                1,
                "pre",
                &current.input_hash,
                &current.model,
                &previous.to_string(),
            )
            .await
            .unwrap();
        assert!(
            !annotations(&store, &target, &planned)
                .await
                .unwrap()
                .contains_key(&1)
        );
        store
            .put_priority(
                1,
                "pre",
                &current.input_hash,
                &current.model,
                &current.payload_json,
            )
            .await
            .unwrap();

        let changed = plan(
            &target,
            vec![mutation(&target, 1, "1 + 2", "1 * 2")],
            Purpose::Pre,
        );
        assert!(
            annotations(&store, &target, &changed)
                .await
                .unwrap()
                .is_empty()
        );

        store
            .add_outcome(Outcome {
                mutant_id: 1,
                status: Status::Uncaught,
                output: "private log".into(),
                time: chrono::Utc::now(),
                duration_ms: 12,
            })
            .await
            .unwrap();
        store
            .add_outcome(Outcome {
                mutant_id: 2,
                status: Status::Timeout,
                output: "private log".into(),
                time: chrono::Utc::now(),
                duration_ms: 12,
            })
            .await
            .unwrap();
        let (client, handle) = server("200 OK").await;
        let post = evaluate(
            &store,
            &client,
            std::slice::from_ref(&target),
            Purpose::Post,
            false,
        )
        .await
        .unwrap();
        assert_eq!(post.evaluated, 1);
        let req = handle.await.unwrap();
        assert!(!req.to_string().contains("private log"));
        assert_eq!(req["state"]["mutants"].as_object().unwrap().len(), 1);
        assert_eq!(req["state"]["mutants"]["m1"]["observed_status"], "Uncaught");
        let post_plan = prepare(&store, &target, Purpose::Post).await.unwrap();
        assert_eq!(
            annotations(&store, &target, &post_plan).await.unwrap()[&1]
                .category
                .as_deref(),
            Some("boundary")
        );
        store
            .add_outcome(Outcome {
                mutant_id: 1,
                status: Status::TestFail,
                output: "caught".into(),
                time: chrono::Utc::now(),
                duration_ms: 20,
            })
            .await
            .unwrap();
        let post_plan = prepare(&store, &target, Purpose::Post).await.unwrap();
        assert!(
            annotations(&store, &target, &post_plan)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn threshold_strictly_excludes_without_writing_outcomes_and_timeout_is_untouched() {
        use crate::core::runner::TestRunner;
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("test.rs");
        let mut target = saved(path.clone(), "fn f() { let a = 1 + 2; let b = 3 + 4; }\n");
        std::fs::write(&path, &target.text).unwrap();
        let store = SqlStore::new("sqlite::memory:".into()).await.unwrap();
        target.id = store.add_target(target.clone()).await.unwrap();
        for (old, new) in [("1 + 2", "1 - 2"), ("3 + 4", "3 - 4")] {
            store
                .add_mutant(mutation(&target, 0, old, new))
                .await
                .unwrap();
        }
        store
            .add_outcome(Outcome {
                mutant_id: 1,
                status: Status::Timeout,
                output: "timeout".into(),
                time: chrono::Utc::now(),
                duration_ms: 42,
            })
            .await
            .unwrap();
        let (client, handle) = server("200 OK").await;
        evaluate(
            &store,
            &client,
            std::slice::from_ref(&target),
            Purpose::Pre,
            false,
        )
        .await
        .unwrap();
        handle.await.unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let registry = Arc::new(crate::LanguageRegistry::new());
        let mut runner = TestRunner::new_with_baseline(
            "true".into(),
            Some(2),
            running.clone(),
            store.clone(),
            true,
            false,
            registry.clone(),
        )
        .await
        .unwrap();
        runner.set_priority_threshold(2.0);
        runner
            .run_mutation_campaign(vec![target.clone()], None)
            .await
            .unwrap();
        assert_eq!(
            store.get_outcome(1).await.unwrap().unwrap().status,
            Status::Timeout
        );
        assert_eq!(
            store.get_outcome(2).await.unwrap().unwrap().status,
            Status::Uncaught
        ); // score == threshold
        assert_eq!(std::fs::read_to_string(&path).unwrap(), target.text);
        // A malformed cached entry also runs, even if the threshold is set.
        let row = store.get_priority(1, "pre").await.unwrap().unwrap();
        store
            .put_priority(1, "pre", &row.input_hash, &row.model, "broken")
            .await
            .unwrap();
        let mut runner = TestRunner::new_with_baseline(
            "true".into(),
            Some(2),
            running.clone(),
            store.clone(),
            true,
            false,
            registry.clone(),
        )
        .await
        .unwrap();
        runner.set_priority_threshold(2.0);
        runner
            .run_mutation_campaign(vec![target.clone()], None)
            .await
            .unwrap();
        assert_eq!(
            store.get_outcome(1).await.unwrap().unwrap().status,
            Status::Uncaught
        );

        // A valid below-threshold judgment is still ignored when the flag is absent.
        store
            .put_priority(1, "pre", &row.input_hash, &row.model, &row.payload_json)
            .await
            .unwrap();
        store
            .add_outcome(Outcome {
                mutant_id: 1,
                status: Status::Timeout,
                output: "retry".into(),
                time: chrono::Utc::now(),
                duration_ms: 42,
            })
            .await
            .unwrap();
        let mut runner = TestRunner::new_with_baseline(
            "true".into(),
            Some(2),
            running,
            store.clone(),
            true,
            false,
            registry,
        )
        .await
        .unwrap();
        runner
            .run_mutation_campaign(vec![target], None)
            .await
            .unwrap();
        assert_eq!(
            store.get_outcome(1).await.unwrap().unwrap().status,
            Status::Uncaught
        );
    }
}
