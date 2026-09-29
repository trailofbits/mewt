//! Versioned, opt-in semantic annotations. A score is a heuristic ordinal rubric
//! position, never a test result or a calibrated TCAP probability.
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::SqlStore;
use crate::core::store::PriorityRow;
use crate::core::typesafe::{self, Answer, Evaluation, EvaluationResult, Question, Usage};
use crate::types::{AppResult, Hash, Mutant, Outcome, Status, Target};

// Experimental product limits, not published TypeSafe API limits. Revise after a pilot.
pub const MAX_SOURCE_LINES: usize = 192;
pub const SOURCE_OVERLAP_LINES: usize = 40;
pub const MAX_SOURCE_BYTES: usize = 16_384;
pub const MAX_REQUEST_BYTES: usize = 32_768;
pub const MAX_MUTANTS_PER_REQUEST: usize = 8;
const VERSION: u32 = 2;
const MODEL: &str = "jev-latest";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Purpose {
    Pre,
    Post,
}
impl Purpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pre => "pre",
            Self::Post => "post",
        }
    }
}

const PRE_RUBRIC: [&str; 5] = [
    "Strong source evidence that the edit has no observable effect; executing tests adds little information",
    "The edit may change behavior, but source evidence suggests limited or redundant test signal",
    "Source context is insufficient or mixed; the value of executing tests is unclear",
    "The edit changes a testable behavior; executing tests could reveal a useful distinction",
    "The edit changes a distinct, consequential behavior; executing tests could reveal an important gap",
];
const POST_RUBRIC: [&str; 5] = [
    "Strong source evidence that no detecting test can distinguish the edit's observable behavior",
    "A detecting test would probably cover only a minor or redundant behavior",
    "The source and Uncaught status give insufficient or mixed evidence of a useful new test goal",
    "A detecting test could check a distinct, meaningful behavior missing from the observed tests",
    "A detecting test could cover a consequential, clearly distinct behavior missing from the observed tests",
];

// The full edit is in state. Keep questions small even for multi-line edits,
// but identify the edit *in the instructions*: question IDs are not sent to Jev.
fn preview(text: &str) -> String {
    const MAX_CHARS: usize = 60;
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(MAX_CHARS).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn questions(mutant: &Mutant, purpose: Purpose) -> BTreeMap<String, Question> {
    let reference = format!(
        "state.mutants.m{} on line {} (byte offset {}, {}: {:?} -> {:?})",
        mutant.id,
        mutant.line_offset + 1,
        mutant.byte_offset,
        mutant.mutation_slug,
        preview(&mutant.old_text),
        preview(&mutant.new_text)
    );
    let instructions = match purpose {
        Purpose::Pre => format!(
            "For {reference}, read the complete edit in state.mutants.m{} and the numbered source window. Rate the value of executing tests on this edit: could the outcome reveal a distinct, observable behavior gap? Use only source evidence, regardless of existing test outcomes. Do not guess whether tests will kill it, or infer coverage, equivalence, or subsumption without evidence. Use level 2 when evidence is insufficient.",
            mutant.id
        ),
        Purpose::Post => format!(
            "For {reference}, read the complete edit and observed Uncaught status in state.mutants.m{} and the numbered source window. Rate the value of a new test that distinguishes this edit: would it check a distinct, meaningful behavior? Uncaught means the observed tests did not detect this edit, not that it is equivalent or that a new test is feasible. Do not invent coverage, subsumption, or dominator evidence. Use level 2 when evidence is insufficient.",
            mutant.id
        ),
    };
    let rubric = if purpose == Purpose::Pre {
        PRE_RUBRIC
    } else {
        POST_RUBRIC
    };
    let mut result = BTreeMap::from([(
        format!("m{}_signal", mutant.id),
        Question::Score {
            instructions: json!(instructions),
            criteria: rubric.into_iter().map(|s| json!(s)).collect(),
        },
    )]);
    if purpose == Purpose::Post {
        result.insert(
            format!("m{}_kind", mutant.id),
            Question::Choice {
                instructions: json!(format!(
                    "For {reference}, read the complete edit in state.mutants.m{} and the numbered source window. Which kind of *new assertion* could best distinguish the original behavior from this Uncaught mutant? Choose a test focus, not a claim about equivalence, subsumption, or existing coverage. If no focus follows from the source, choose unclear.",
                    mutant.id
                )),
                criteria: BTreeMap::from([
                    ("boundary".into(), json!("A threshold, range, equality, or off-by-one input at a boundary")),
                    ("branch".into(), json!("A control-flow decision or short-circuit path, other than a numeric boundary")),
                    ("error_path".into(), json!("An error, revert, exception, or validation failure")),
                    ("side_effect".into(), json!("An observable state change, event, I/O, or other side effect")),
                    ("value".into(), json!("A returned value, computation, or argument passed to another function")),
                    ("unclear".into(), json!("No specific distinguishing assertion can be inferred from this source window")),
                ]),
            },
        );
    }
    result
}

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    version: u32,
    answers: BTreeMap<String, Answer>,
    usage: Usage,
}

#[derive(Clone, Debug, Serialize)]
pub struct Annotation {
    pub score: f64,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

pub struct Batch {
    pub request: Evaluation,
    pub mutants: Vec<Mutant>,
}

pub struct Plan {
    purpose: Purpose,
    pub batches: Vec<Batch>,
    pub unsupported: Vec<(i64, &'static str)>,
}

fn source_lines(source: &str) -> Vec<(usize, usize)> {
    let mut start = 0;
    source
        .split_inclusive('\n')
        .map(|line| {
            let end = start + line.len();
            let span = (start, end);
            start = end;
            span
        })
        .collect()
}

fn window_end(lines: &[(usize, usize)], start: usize) -> usize {
    let mut end = start;
    let mut bytes = 0;
    while end < lines.len() && end - start < MAX_SOURCE_LINES {
        let numbered_line_bytes = (end + 1).to_string().len() + 3 + lines[end].1 - lines[end].0;
        if bytes + numbered_line_bytes > MAX_SOURCE_BYTES {
            break;
        }
        bytes += numbered_line_bytes;
        end += 1;
    }
    end
}

fn numbered_source(target: &Target, lines: &[(usize, usize)], start: usize, end: usize) -> String {
    let mut text = String::new();
    for (index, &(from, to)) in lines.iter().enumerate().take(end).skip(start) {
        text.push_str(&format!("{} | ", index + 1));
        text.push_str(&target.text[from..to]);
    }
    text
}

fn language_hint(target: &Target) -> String {
    let label = target.language.to_string();
    let extension = target.path.extension().and_then(|ext| ext.to_str());
    match (label.as_str(), extension) {
        // Older campaigns saved a family-only label even for TypeScript/JSX.
        ("javascript", Some(dialect @ ("js" | "jsx" | "ts" | "tsx"))) => {
            format!("javascript/{dialect}")
        }
        ("suimove", _) => "move/sui".into(),
        _ => label,
    }
}

fn entry(mutant: &Mutant, purpose: Purpose) -> Value {
    let mut item = json!({
        "line": mutant.line_offset + 1,
        "byte_offset": mutant.byte_offset,
        "operator": mutant.mutation_slug,
        "old_text": mutant.old_text,
        "new_text": mutant.new_text,
    });
    if purpose == Purpose::Post {
        // Deliberately do not send test logs, time, duration, paths, or invented failure kinds.
        item["observed_status"] = json!("Uncaught");
    }
    item
}

fn build_request(
    target: &Target,
    lines: &[(usize, usize)],
    start: usize,
    end: usize,
    mutants: &[Mutant],
    purpose: Purpose,
) -> Evaluation {
    let manifest: BTreeMap<String, Value> = mutants
        .iter()
        .map(|m| (format!("m{}", m.id), entry(m, purpose)))
        .collect();
    let mut questions = BTreeMap::new();
    for mutant in mutants {
        questions.extend(self::questions(mutant, purpose));
    }
    let mut evaluation = Evaluation::new(
        json!({
            "purpose": purpose.as_str(),
            "language": language_hint(target),
            "source_window": {
                "first_line": start + 1,
                "text": numbered_source(target, lines, start, end),
            },
            "mutants": manifest,
        }),
        questions,
    );
    evaluation.model = MODEL.into();
    evaluation
}

/// Partition each saved edit into one primary window. Overlap provides context,
/// never duplicate questions. Oversized/invalid edits get no score (fail open).
pub fn plan(target: &Target, mut mutants: Vec<Mutant>, purpose: Purpose) -> Plan {
    let lines = source_lines(&target.text);
    mutants.sort_by_key(|m| (m.byte_offset, m.id));
    let mut batches = Vec::new();
    let mut unsupported = Vec::new();
    let mut seen = HashSet::new();
    let mut pending = Vec::new();
    for mutant in mutants {
        if !seen.insert(mutant.id) {
            continue;
        }
        let offset = mutant.byte_offset as usize;
        let Some(after) = offset.checked_add(mutant.old_text.len()) else {
            unsupported.push((mutant.id, "invalid edit offset"));
            continue;
        };
        if target.text.get(offset..after) != Some(mutant.old_text.as_str()) || lines.is_empty() {
            unsupported.push((mutant.id, "edit does not match saved source"));
            continue;
        }
        let line = lines
            .partition_point(|&(_, end)| end <= offset)
            .min(lines.len() - 1);
        // The saved line location must refer to the source, not just the question ID.
        if mutant.line_offset as usize != line {
            unsupported.push((mutant.id, "edit line does not match saved source"));
            continue;
        }
        pending.push((mutant, line, after));
    }

    let mut index = 0;
    while index < pending.len() {
        let line = pending[index].1;
        let mut start = line.saturating_sub(SOURCE_OVERLAP_LINES);
        let mut end = window_end(&lines, start);
        if end <= line || lines[end - 1].1 < pending[index].2 {
            // A large preceding line or boundary-spanning edit: center on the edit.
            start = line;
            end = window_end(&lines, start);
        }
        if end <= line || lines[end - 1].1 < pending[index].2 {
            unsupported.push((
                pending[index].0.id,
                "edit cannot fit in a bounded source window",
            ));
            index += 1;
            continue;
        }
        let mut group = Vec::new();
        while index < pending.len() {
            let (mutant, mutant_line, after) = &pending[index];
            if *mutant_line >= end || *after > lines[end - 1].1 {
                break;
            }
            group.push(mutant.clone());
            index += 1;
        }
        let mut chunk = Vec::new();
        for mutant in group {
            let mut candidate = chunk.clone();
            candidate.push(mutant.clone());
            let request = build_request(target, &lines, start, end, &candidate, purpose);
            if candidate.len() > MAX_MUTANTS_PER_REQUEST
                || !serde_json::to_vec(&request).is_ok_and(|v| v.len() <= MAX_REQUEST_BYTES)
            {
                if !chunk.is_empty() {
                    batches.push(Batch {
                        request: build_request(target, &lines, start, end, &chunk, purpose),
                        mutants: std::mem::take(&mut chunk),
                    });
                }
                let single = build_request(
                    target,
                    &lines,
                    start,
                    end,
                    std::slice::from_ref(&mutant),
                    purpose,
                );
                if serde_json::to_vec(&single).is_ok_and(|v| v.len() <= MAX_REQUEST_BYTES) {
                    chunk.push(mutant);
                } else {
                    unsupported.push((mutant.id, "edit and context exceed request byte limit"));
                }
            } else {
                chunk.push(mutant);
            }
        }
        if !chunk.is_empty() {
            batches.push(Batch {
                request: build_request(target, &lines, start, end, &chunk, purpose),
                mutants: chunk,
            });
        }
    }
    Plan {
        purpose,
        batches,
        unsupported,
    }
}

pub async fn prepare(store: &SqlStore, target: &Target, purpose: Purpose) -> AppResult<Plan> {
    let mut mutants = store.get_mutants(target.id).await?;
    if purpose == Purpose::Post {
        let outcomes: HashMap<i64, Outcome> = store
            .get_outcomes(target.id)
            .await?
            .into_iter()
            .map(|o| (o.mutant_id, o))
            .collect();
        mutants.retain(|m| {
            outcomes
                .get(&m.id)
                .is_some_and(|o| o.status == Status::Uncaught)
        });
    }
    Ok(plan(target, mutants, purpose))
}

fn fingerprint(
    target: &Target,
    mutant: &Mutant,
    purpose: Purpose,
    request: &Evaluation,
) -> AppResult<String> {
    // Include the *whole* shared state and every question: companions can affect an answer.
    // BTreeMap question/manifest keys give deterministic serialization.
    Ok(Hash::digest(serde_json::to_string(&(
        VERSION,
        purpose.as_str(),
        target.id,
        &target.file_hash,
        mutant,
        request,
    ))?)
    .to_hex())
}

fn checked_annotation(
    row: &PriorityRow,
    hash: &str,
    mutant: &Mutant,
    purpose: Purpose,
    request: &Evaluation,
) -> Option<Annotation> {
    if row.input_hash != hash || row.model.trim().is_empty() {
        return None;
    }
    let payload: Payload = serde_json::from_str(&row.payload_json).ok()?;
    if payload.version != VERSION {
        return None;
    }
    let expected = questions(mutant, purpose);
    if payload.answers.keys().ne(expected.keys()) {
        return None;
    }
    let single = Evaluation {
        state: request.state.clone(),
        model: request.model.clone(),
        questions: expected,
    };
    let result = EvaluationResult {
        model: row.model.clone(),
        answers: payload.answers,
        usage: payload.usage,
    };
    typesafe::validate_answers(&single, &result).ok()?;
    let Answer::Score {
        score, confidence, ..
    } = result.answers.get(&format!("m{}_signal", mutant.id))?
    else {
        return None;
    };
    let category = if purpose == Purpose::Post {
        match result.answers.get(&format!("m{}_kind", mutant.id))? {
            Answer::Choice { choice, .. } => Some(choice.clone()),
            _ => return None,
        }
    } else {
        None
    };
    Some(Annotation {
        score: *score,
        confidence: *confidence,
        category,
    })
}

pub async fn annotations(
    store: &SqlStore,
    target: &Target,
    plan: &Plan,
) -> AppResult<HashMap<i64, Annotation>> {
    let mut found = HashMap::new();
    for batch in &plan.batches {
        for mutant in &batch.mutants {
            let hash = fingerprint(target, mutant, plan.purpose, &batch.request)?;
            if let Some(row) = store.get_priority(mutant.id, plan.purpose.as_str()).await? {
                if let Some(annotation) =
                    checked_annotation(&row, &hash, mutant, plan.purpose, &batch.request)
                {
                    found.insert(mutant.id, annotation);
                }
            }
        }
    }
    Ok(found)
}

/// Return a current cached annotation for one mutant in a planned batch.
pub(crate) async fn cached_annotation(
    store: &SqlStore,
    target: &Target,
    mutant: &Mutant,
    purpose: Purpose,
    request: &Evaluation,
) -> AppResult<Option<Annotation>> {
    let hash = fingerprint(target, mutant, purpose, request)?;
    let Some(row) = store.get_priority(mutant.id, purpose.as_str()).await? else {
        return Ok(None);
    };
    Ok(checked_annotation(&row, &hash, mutant, purpose, request))
}

/// Validate and persist one answer from a shared TypeSafe response.
pub(crate) async fn store_annotation(
    store: &SqlStore,
    target: &Target,
    mutant: &Mutant,
    purpose: Purpose,
    request: &Evaluation,
    result: &EvaluationResult,
) -> AppResult<Option<Annotation>> {
    let ids = questions(mutant, purpose);
    let answers = ids
        .keys()
        .filter_map(|id| {
            result
                .answers
                .get(id)
                .map(|answer| (id.clone(), answer.clone()))
        })
        .collect();
    let payload = Payload {
        version: VERSION,
        answers,
        usage: result.usage.clone(),
    };
    let row = PriorityRow {
        input_hash: fingerprint(target, mutant, purpose, request)?,
        model: result.model.clone(),
        payload_json: serde_json::to_string(&payload)?,
    };
    let Some(annotation) = checked_annotation(&row, &row.input_hash, mutant, purpose, request)
    else {
        return Ok(None);
    };
    store
        .put_priority(
            mutant.id,
            purpose.as_str(),
            &row.input_hash,
            &row.model,
            &row.payload_json,
        )
        .await?;
    Ok(Some(annotation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    #[test]
    fn windows_and_batches_are_bounded_and_unique() {
        let text: String = (0..500).map(|i| format!("let v{i} = 1 + 2;\n")).collect();
        let target = saved("many.rs".into(), &text);
        let mutants: Vec<_> = (0..500)
            .step_by(7)
            .enumerate()
            .map(|(id, line)| {
                let offset = text.match_indices("1 + 2").nth(line).unwrap().0;
                Mutant {
                    id: id as i64 + 1,
                    target_id: 1,
                    byte_offset: offset as u32,
                    line_offset: line as u32,
                    old_text: "1 + 2".into(),
                    new_text: "1 - 2".into(),
                    mutation_slug: "AOS".into(),
                }
            })
            .collect();
        let planned = plan(&target, mutants.clone(), Purpose::Pre);
        assert!(planned.unsupported.is_empty(), "{:?}", planned.unsupported);
        let mut seen = HashSet::new();
        for batch in planned.batches {
            assert!(batch.mutants.len() <= MAX_MUTANTS_PER_REQUEST);
            assert!(serde_json::to_vec(&batch.request).unwrap().len() <= MAX_REQUEST_BYTES);
            let source = &batch.request.state["source_window"];
            assert!(source["text"].as_str().unwrap().len() <= MAX_SOURCE_BYTES);
            assert!(source["text"].as_str().unwrap().lines().count() <= MAX_SOURCE_LINES);
            assert_eq!(
                batch.request.state["mutants"].as_object().unwrap().len(),
                batch.mutants.len()
            );
            for m in batch.mutants {
                assert!(seen.insert(m.id));
            }
        }
        assert_eq!(seen.len(), mutants.len());

        let huge = saved(
            "huge.rs".into(),
            &format!("{}1 + 2\n", "x".repeat(MAX_SOURCE_BYTES)),
        );
        let excluded = plan(
            &huge,
            vec![mutation(&huge, 777, "1 + 2", "1 - 2")],
            Purpose::Pre,
        );
        assert_eq!(excluded.unsupported.len(), 1);
        assert!(excluded.batches.is_empty());
        let following = saved(
            "following.rs".into(),
            &format!("{}\nlet x = 1 + 2;\n", "x".repeat(MAX_SOURCE_BYTES)),
        );
        let planned = plan(
            &following,
            vec![mutation(&following, 1, "1 + 2", "1 - 2")],
            Purpose::Pre,
        );
        assert!(planned.unsupported.is_empty(), "{:?}", planned.unsupported);
        assert_eq!(planned.batches.len(), 1); // Large prior line is omitted, not the edit.
        let invalid = plan(
            &target,
            vec![
                mutation(&target, 800, "1 + 2", "1 - 2"),
                mutation(&target, 801, "1 + 2", "1 - 2"),
            ],
            Purpose::Pre,
        );
        assert_eq!(
            invalid
                .batches
                .iter()
                .map(|b| b.mutants.len())
                .sum::<usize>(),
            2
        );
    }

    #[test]
    fn long_edits_and_legacy_dialects_keep_the_full_edit_in_a_bounded_request() {
        let body = "let value = 1 + 2;\n".repeat(177);
        let mut target = saved("example.ts".into(), &body);
        target.language = "JavaScript".parse().unwrap();
        let planned = plan(
            &target,
            vec![mutation(&target, 1, &body, "return 0;\n")],
            Purpose::Post,
        );
        assert!(planned.unsupported.is_empty(), "{:?}", planned.unsupported);
        let batch = &planned.batches[0];
        assert_eq!(batch.request.state["language"], "javascript/ts");
        assert_eq!(batch.request.state["mutants"]["m1"]["old_text"], body);
        assert!(
            batch.request.state["source_window"]["text"]
                .as_str()
                .unwrap()
                .contains("177 | let value")
        );
        assert!(serde_json::to_vec(&batch.request).unwrap().len() <= MAX_REQUEST_BYTES);
        let instructions = serde_json::to_string(&batch.request.questions).unwrap();
        assert!(instructions.contains("state.mutants.m1"));
        assert!(!instructions.contains(&body)); // Full edit is in state, not repeated in each question.

        target.language = "SuiMove".parse().unwrap();
        let move_plan = plan(
            &target,
            vec![mutation(&target, 1, "1 + 2", "1 - 2")],
            Purpose::Pre,
        );
        assert_eq!(move_plan.batches[0].request.state["language"], "move/sui");
    }
}
