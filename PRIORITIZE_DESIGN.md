# Design: `mewt prioritize`

Status: proposal for implementation. This document specifies the agreed product behavior and identifies experiments still needed. [`docs/tcap-prioritization.md`](docs/tcap-prioritization.md) contains the earlier research and exploratory per-mutant query; **this document governs the command design where they differ**. No API budgets, thresholds, or score calibration have been validated yet.

## Goals and scope

There are two separate decisions:

1. **Pre-campaign:** Which saved mutants are unlikely to justify the cost of executing tests? Optionally use a user-specified threshold to *exclude* only mutants with a valid below-threshold judgment. **No judgment means normal behavior: run the mutant.**
2. **Post-campaign:** Among mutants with an `Uncaught` outcome, which would be valuable goals for writing new tests? Show advisory annotations alongside results. No post-campaign score changes test execution.

Use Jev for semantic judgments, not for mutation generation, test execution, or reporting observed test status. Do not call either score “TCAP”: Kaufman et al., “Prioritizing Mutants to Guide Mutation Testing,” ICSE 2022 ([paper PDF](prioritizing_mutants_tcap_icse_2022.pdf), [DOI](https://doi.org/10.1145/3510003.3510187)) define TCAP in terms of the probability a test elicited by a mutant advances test completeness. Equivalent mutants have TCAP 0, dominators 1, and subsumed mutants can still be valuable. Their estimation uses detection matrices and static program context on Java subjects. Jev's ordinal rubric is not their calibrated estimate, cannot prove equivalence/dominator status, and is not necessarily transferable to Mewt's languages. The TCAP *objective* motivates post-campaign ranking; pre-campaign execution-value filtering is a different objective and needs its own rubric.

## Relevant existing behavior

- `mewt mutate` saves targets and mutants; `mewt run` can run saved mutants without generating again. A pre-filter can initially use the explicit sequence `mewt mutate`, `mewt prioritize mutants`, `mewt run --priority-threshold …`. Do not require `run` to contact TypeSafe or require a key.
- In `migrations/001_init.sql`, `targets` contains source `text`, `language`, and unique `file_hash`; `mutants` contains `target_id`, byte/line offsets, old/new text, and mutation slug. A mutant ID identifies its saved source/edit but not the query version or test evidence.
- `outcomes.mutant_id` is already the **primary key**; `SqlStore::add_outcome` updates its status, output, time, and duration on retest. There is no need to add `outcome_id`: a pointer to this mutable row cannot distinguish its previous and current contents. Status variants include `Uncaught`, `TestFail`, `Skipped`, and `Timeout`; post-campaign eligibility is **only `Uncaught`**.
- `mewt run` has a separate severity-based optimization that skips less severe mutants after an uncaught mutation on the same line. Priority exclusions must not be confused with or stored as these `Skipped` outcomes. `mewt results` currently supports normal table/JSON and other output modes and defaults to showing uncaught mutants.
- `src/typesafe.rs` already provides a generic async System One HTTP client with Score, Choice, and Noul questions, response validation, and bounded retry for 429/529. Build prompts, caching, and campaign policy above that client; keep it reusable for future features.

## CLI contract

Proposed syntax (names subject to CLI review, rather than silently giving ambiguous bare `prioritize` a default):

```text
mewt mutate <targets>
mewt prioritize mutants [TARGET …] [--force]   # pre-campaign annotation
mewt run [--priority-threshold T]              # execution using cached pre scores
mewt prioritize survivors [TARGET …] [--force] # post-campaign annotation
mewt results [existing flags]                  # display cached post annotations
```

Both `prioritize` modes operate on **saved targets/mutants** and do not themselves generate mutations or execute tests. Reuse existing target filtering conventions; confirm no-filter behavior (prefer all saved targets) during CLI review. `prioritize mutants` assesses candidates without reading test outcomes, including when an outcome happens to exist; its answer must be comparable across pre/post test runs with otherwise identical input. `prioritize survivors` only assesses mutants with a *current* `Uncaught` outcome, potentially using the observed status and bounded, relevant output evidence. For `--force`, act as if **no selected eligible mutant has a cache entry**: re-query all, replace successful validated entries, preserve prior data on failures, and do not present stale data as fresh.

**Immediately** check `TYPESAFE_API_KEY` for either `prioritize` mode, **before cache lookup**, even if no API request would ultimately be needed. A missing/blank key fails early with a link to [TypeSafe](https://typesafe.ai/) to sign up/get a key and a notice that code will be sent to the service. Put that notice in command help and docs too. No other command, especially `run` or `results`, checks for the key or accesses TypeSafe. Avoid printing keys, source bodies, or full service error bodies in logs.

### Threshold semantics: conservative exclusion only

`--priority-threshold T` belongs on `mewt run`; calculating judgments and applying campaign policy are separate steps. Start with an experimental Score scale **0–4** (five ordered rubric levels). Require finite `T` within this range; `score < T` excludes, `score == T` runs. There is **no default threshold**: without the flag, preserve current behavior even if annotations exist. A valid, fresh pre-campaign judgment is the **only** reason to exclude. If the entry is missing, stale, malformed, unsupported, failed to load, or otherwise not trustworthy, run the mutant. Do not use category or confidence alone as a skip signal, and do not treat service failure as a zero score. A cache read/storage error should be reported rather than silently hiding a mutant; if the runner can continue safely, its selection must fail open.

A priority exclusion is *not* an execution outcome: do not insert a `Status::Skipped` row. Keep the mutant eligible for subsequent runs without the threshold. Report priority-excluded counts separately from severity-skipped and actually tested counts. Keep existing severity-ordering and line-skip logic for mutants that remain eligible. The exact treatment of timeout retests under a valid pre judgment needs an explicit implementation decision; **missing judgment still runs**. Initially require pre-generated saved mutants for threshold filtering; determine later whether a single `run` invocation should generate, query Jev, and test, rather than silently skipping the pre-step.

### Display contract

Show fresh post-campaign annotations next to `Uncaught` results if present; otherwise display results as today, without a key or network access. Initially keep existing result order and filters. Include an optional score and uncertainty/category where applicable, without interpreting missing annotation as zero. Preserve existing formats (especially JSON field meanings, IDs-only output, and SARIF); decide whether/how to add optional annotations to JSON/table/SARIF before coding. Later filter/sort flags are separate work. A prior post annotation for a mutant that is now caught must not appear as a current uncaught result.

## Jev questions: two purposes, not one universal score

[TypeSafe's HTTP API](https://docs.typesafe.ai/api.md) accepts a `state` and multiple independently evaluated typed `questions` over that same state (`POST /v1/systemone`, bearer authentication). State may be structured JSON. The question ID routes the answer to code but **is not sent to the model**; each question's `instructions` must identify the mutant and edit it means. Score returns an ordinal probability-weighted position over specified levels, per-level probabilities and distribution-derived confidence. Confidence is **not** a statistical confidence interval, probability of correctness, or an additional estimate of TCAP. Do not ask Jev a second vague “how confident are you?” question. See [State](https://docs.typesafe.ai/concepts/state.md), [Score](https://docs.typesafe.ai/primitives/score.md), and [Choice](https://docs.typesafe.ai/primitives/choice.md).

An illustrative request (not a tested final rubric):

```json
{
  "model": "jev-latest",
  "state": {
    "language": "rust",
    "source_window": {"first_line": 1, "text": "<original source with absolute line numbers>"},
    "mutants": {
      "m34": {"line": 56, "byte_offset": 912, "operator": "<slug>", "old_text": "a + b", "new_text": "a - b"}
    },
    "purpose": "pre"
  },
  "questions": {
    "m34_signal": {
      "type": "score",
      "instructions": "For state.mutants.m34 on line 56 (a + b -> a - b), if we execute tests on this mutant, how likely is the result to provide actionable, nonredundant information about a meaningful behavior difference? Use source context only; do not assume any test outcome.",
      "criteria": [
        "Very unlikely to provide actionable, nonredundant signal",
        "Unlikely to provide actionable, nonredundant signal",
        "Unclear or mixed evidence of actionable signal",
        "Likely to provide actionable, nonredundant signal",
        "Very likely to provide actionable, nonredundant signal"
      ]
    }
  }
}
```

**Pre question:** estimate value *per test execution*, not how easy a mutant will be to kill, its fault likelihood, or code importance. A low score is a heuristic reason to consider excluding it, not proof it is equivalent or redundant. Threshold policy is user-controlled and must be measured against missed signal and runtime saved.

**Post question:** using actual `Uncaught` evidence, estimate how useful the mutant would be as a *new test goal*: would writing a detecting test likely advance completeness rather than repeat existing distinctions? Use its own 0–4 low-to-high rubric, but do not compare numeric scores across purposes. Optional Choice categories could be suspected `trivial`, `equivalent`, `other_or_subsumed`, and `unclear`; `dominator` needs mutant-set/subsumption evidence and should not be guessed from a diff. Choice is a primary display annotation, not a mutually exclusive formal classification. Decide whether the extra categorization justifies cost in a pilot. `Uncaught` does not prove equivalence, and a source diff does not prove triviality. The earlier document's single post-only query is background, not the pre-campaign rubric.

Each question must name its state entry and its edit/line, not merely have an informative ID. Validate returned IDs, types, ranges and distributions; associate results with the saved mutant ID rather than guessing from response order. Do not send invented coverage, subsumption, or failure types. Include language/dialect. Keep stored observed test facts distinct from heuristic answers.

## File windows, batching and privacy

Prefer sharing source code once per request and asking mutant-specific questions, instead of repeating overlapping source snippets per mutant. Supply a manifest containing only mutants assigned to that request. All questions see the same state and are independent of each other's answers. Multiple questions per mutant (Score and, if adopted, Choice) increase output size: a request per file is a *goal*, not a guarantee it fits service limits.

For large files, use overlapping source windows with absolute line numbers, and assign every eligible mutant to **exactly one primary window** (overlap is for context, not duplicate judgments). For example, a 2,000-line file might yield windows 1–1,200 and 801–2,000, with disjoint mutant assignments. Center a window on a mutant spanning a boundary when needed. Apply named, tunable limits such as `MAX_SOURCE_LINES`, `SOURCE_OVERLAP_LINES`, `MAX_REQUEST_BYTES`, and `MAX_MUTANTS_PER_REQUEST`. **1,200 is an illustration, not a validated default.** Bound both lines and bytes/token estimate; generated files can contain enormous single lines. Split large mutant batches further, use bounded concurrency, and fail/report rather than silently truncating a mutant or accepting ambiguous answer attribution. Measure request limits, cost, latency and answer quality on actual multi-language targets before choosing constants.

Opt-in prioritization sends source windows, mutation edits and (for post mode) selected outcome evidence to TypeSafe. Source may be private; the help and missing-key message must say so. Limit/sanitize outcome output, avoid sending whole test logs by default, never persist/log credentials, and do not claim whole-file insight for judgments based on a partial window. Offer a safe fallback for regions that cannot fit configured limits rather than making a huge request.

## Cache: proposed new table

Use one new table for both purposes, **not** JSON columns on `outcomes` and not a new `outcome_id`. Suggested stable schema (precise SQL naming subject to migration review):

```text
mutant_id     INTEGER NOT NULL REFERENCES mutants(id) ON DELETE CASCADE
purpose       TEXT    NOT NULL                 -- 'pre' | 'post'
input_hash    TEXT    NOT NULL                 -- canonical input fingerprint
model         TEXT    NOT NULL                 -- concrete model returned
created_at    TEXT    NOT NULL
payload_json  TEXT    NOT NULL                 -- versioned answers and metadata
PRIMARY KEY (mutant_id, purpose)
```

A payload version allows the JSON answer shape to change without another migration. Persist validated score, distributions/confidence, any Choice, returned model and usage, plus minimal sanitized provenance; don't store the API key, full file or unbounded test output by default. FK cascade handles mutant deletion. One row per `(mutant_id, purpose)` lets future pre and post judgments coexist; don't make an `outcomes` row just to hold a pre judgment. Upsert a row only on validated success, keeping the old row on a failed forced refresh. Unknown payload versions are cache misses, never zero scores.

Compute `input_hash` over canonical, versioned **semantic inputs**: purpose; question template and full rubric (or hash thereof); requested model name; language; saved mutant identity/edit/location; exact source window and absolute range; and, for post, precisely the outcome evidence provided to Jev. **Do not hash a mere outcome pointer:** `add_outcome` updates the same PK row on retest. Avoid hashing time/duration unless they are actually supplied to Jev. A threshold change is a policy change and must *not* invalidate a judgment. Since other mutants in a shared state might affect an answer, conservatively hash the exact shared state plus the question for each answer; changes to batch composition may cause additional queries but must not cause unsound cache reuse. Use deterministic serialization (e.g., sorted keys) so equivalent requests hash identically.

A valid cache hit requires matching purpose and input hash, supported payload schema, matching expected question types/IDs, finite in-range score and valid answer distribution; `post` additionally requires a current `Uncaught` outcome and matching included evidence. Corrupt/stale entries are misses. A response may name a concrete model different from `jev-latest`; record it. An existing alias-based cache cannot detect an upstream model revision when the alias string remains unchanged. Document that and use `--force` to refresh, or pin/check concrete models in a later iteration. Do not present an old row as fresh after its input changes.

## Errors, rollout and verification

- Key check before cache on `prioritize`, with TypeSafe sign-up link and source-upload notice. No TypeSafe dependency for normal mutation testing, `run`, or `results`.
- Bound retries for 429/529; surface 401/422, transport failure, missing/mismatched answers and storage errors. Successful chunks may be saved; an incomplete `prioritize` exits nonzero with evaluated/cached/failed counts and can resume next time. Never interpret a failed query as a below-threshold judgment.
- Preserve all original results and test outcomes. On `run`, only a validated fresh pre score strictly below the user's threshold excludes a mutant; all missing/invalid/stale cases run. Test this boundary and `--force` replacement/failure behavior explicitly.
- Pilot on saved examples across Mewt's supported languages, including large files, difficult equivalent-like changes, trivial-like changes, survivors and ambiguous cases. Compare excluded mutants against missed useful signal and test time saved; evaluate post rankings against tests developers actually write. Treat scores as heuristic until validated on Mewt data.
- Implement in stages: approved migration/store and cache validation; versioned rubrics and window builder; two explicit prioritize commands; `run` threshold; optional `results` annotations. Follow `AGENTS.md`: **obtain explicit approval before modifying `migrations/`**, run `just reset-db` after schema/SQL changes (protect existing local DB data first), and run `just pre-commit`, build and tests for implementation batches. No schema or SQL is changed by this document.

**Still to settle before coding:** exact rubric text and whether to include Choice in v1; concrete line/byte/question and concurrency caps; outcome fields safe and useful to send; no-target filter default; JSON/SARIF display shape; timeout-retest filtering policy; and whether `run` should support a single-call generate/prioritize/test workflow. Do not fill these in with asserted API limits or an uncalibrated “TCAP probability.”
