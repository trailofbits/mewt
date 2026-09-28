# TCAP-inspired mutant prioritization with Jev

## Purpose and scope

This document proposes an optional `mewt prioritize` command that annotates and ranks existing Mewt mutants using TypeSafe's Jev API. The proposal is informed by the paper cited below:

> Samuel J. Kaufman, Ryan Featherman, Justin Alvin, Bob Kurtz, Paul Ammann, and René Just. “Prioritizing Mutants to Guide Mutation Testing.” ICSE 2022. [DOI: 10.1145/3510003.3510187](https://doi.org/10.1145/3510003.3510187).

The command would be advisory postprocessing. It would not change mutation generation, execute tests, or replace Mewt's existing outcomes and severity. Its intended product is a useful review order and explicit, uncertain semantic annotations—not a claim that Jev has established a mutant's formal status or measured its actual TCAP.

## Paper findings applicable to `mewt prioritize`

The paper distinguishes **mutation analysis** from **mutation testing**. Mutation analysis asks how adequate an existing test set is. Mutation testing presents mutants incrementally as concrete test goals, with the aim of eliciting tests that improve test quality. The paper argues that selecting a representative subset once and measuring its mutation score does not capture the effort or progress of this incremental workflow.

The paper introduces **test completeness advancement probability (TCAP)**: for a mutant presented as a test goal, the probability that the resulting test advances test completeness. Test completeness is approximated using dominator score, not ordinary mutation score. The paper's key cases are:

- **Equivalent mutants:** TCAP 0; no test can distinguish the mutant from the original.
- **Dominator mutants:** TCAP 1; detecting one advances completeness by definition in the paper's model.
- **Subsumed mutants:** TCAP is greater than 0 and at most 1. Subsumed does not mean useless: a mutant may have TCAP 1 if every test detecting it also detects a dominator.

The central practical insight is that a mutant's value as a test goal depends on the tests it elicits and their effect on completeness—not only on whether the mutant can be killed, how dramatic its source change looks, or whether it is fault-coupled. The paper predicts TCAP using **static program-context features** and evaluates incremental selection by simulated work. Its model features include mutation operator, AST node/type/context, parent/child context, relative position, and nesting. Its study reports that TCAP-prioritized selection improved test completeness more rapidly than random selection in its studied projects.

Implications for Mewt:

1. Ranking is a natural feature for an incremental-testing-oriented command; a plain mutation score is not a ranking signal for which test goal to tackle next.
2. The estimate should be framed as the chance that a mutant is a useful *test goal*, rather than generic bug likelihood, code importance, or developer effort.
3. A useful prediction can use mutation and source context, but Mewt must distinguish contextual inference from evidence obtained by running tests.
4. One-mutant-at-a-time results are useful only as a starting ranking. In the paper's workflow, each new test may kill the selected mutant and subsumed mutants, changing what remains valuable. A static one-shot ordering will not account for those interactions unless Mewt refreshes it as evidence changes.

The paper's empirical results are not guarantees for Mewt: its dataset and evaluation use Java projects and a particular mutation framework, and the paper explicitly discusses validity threats around project/language generalization and approximating equivalence and dominator relationships from finite test suites.

## TypeSafe / Jev: capabilities, constraints, and fit

TypeSafe describes **System One** as an API for evaluating supplied state against narrow, typed questions. Its flagship model, Jev, returns structured judgments rather than free-form explanations. The documented question primitives are:

- **Score:** evaluate content against an ordered rubric; returns a weighted score, level probabilities, and confidence.
- **Choice:** select one of a fixed set of options; returns the selected option, option probabilities, and confidence.
- **Noul:** estimate the probability of a yes/no proposition.

A single request can contain multiple questions over the same state. That suits per-mutant annotation: Mewt can request a TCAP-like priority score and a separate category in one evaluation. Typed answers simplify parsing, but do **not** guarantee the judgment is true or calibrated for Mewt's domain. The API documents the `jev-latest` model alias and the HTTP endpoint `POST https://api.typesafe.ai/v1/systemone`, authenticated with a bearer API key. The documented service errors include authentication/validation errors, rate limits, and temporary overload; retry/backoff is appropriate for 429 and 529 responses.

The current TypeSafe documentation lists Python and JavaScript SDKs, but no Rust SDK. That is not a blocker: Mewt is Rust, and can call the documented HTTP/JSON API directly with an HTTP client and deserialize the response. It should validate response shape and question IDs, handle API/service failures, and keep credentials out of logs. Relevant live documentation:

- [TypeSafe API](https://docs.typesafe.ai/api.md)
- [System One](https://docs.typesafe.ai/concepts/system-one.md)
- [Choice](https://docs.typesafe.ai/primitives/choice.md)
- [Score](https://docs.typesafe.ai/primitives/score.md)
- [Confidence](https://docs.typesafe.ai/confidence.md)
- [Python SDK](https://docs.typesafe.ai/sdk/python.md) (shows the SDK surface; not a Rust SDK)

### Suitability for Mewt

The API is a plausible fit for an **opt-in, best-effort annotation/ranking command**. A per-mutant state can include the source diff and context Mewt already knows; Score and Choice are appropriate for the two distinct outputs. Batching several questions per request avoids separate calls for independent judgments.

Important constraints for product design:

- **Not an empirical TCAP calculator.** Jev's contextual judgment is not the paper's test-matrix-derived TCAP estimate. Name and document it as `TCAP-like`, `estimated test-goal value`, or similarly qualified unless it is later trained/validated against an operational Mewt dataset.
- **Evidence matters.** A source diff alone cannot establish test coverage, killability, equivalence, triviality, or subsumption. Supply actual Mewt results when available and represent unavailable facts explicitly as unknown; never invite Jev to fill in missing facts.
- **Privacy and consent.** Source snippets and mutation details leave the machine when sent to TypeSafe. Make the operation explicit and opt-in, disclose what is sent, and avoid transmitting more context than needed. Keep the API key server-side/environment-provided and never persist it in result output.
- **Failure must not corrupt ordinary results.** No key, user opt-out, network failure, timeout, rate limit, malformed response, or API outage should leave Mewt's unannotated results available. Report skipped/failed prioritization distinctly; do not silently treat an API failure as a low score.
- **Cost and scale.** A request per mutant may be expensive or slow for large campaigns. Measure request/token cost and latency on representative runs. Batch multiple mutants into one state only if the API's request limits and answer semantics are verified and each mutant remains independently attributable; otherwise use bounded concurrency and batches of mutant evaluations. Add a clear limit/filter option if needed.
- **Repeatability.** Persist the raw judgment, model identifier, rubric/version, and enough mutant identity to associate an annotation with the exact input. Rankings may change with model aliases, prompts, or source changes. Do not overwrite test outcomes.
- **Freshness.** Re-evaluate when the mutant, source context, or relevant test evidence changes. If new tests are written, old rankings may no longer reflect the remaining mutant set.

Mewt currently supports multiple languages and stores mutation/test campaign data. The command should reuse existing mutant identity, source-context, and outcome abstractions where practical, but the prompt must not assume Java-specific operators or terminology. Exact integration details—such as whether prioritization reads generated mutants, persisted campaign mutants, or both—need to follow the command's eventual product contract.

## Proposed per-mutant Jev query

The following is a draft HTTP API request shape, not a claim that it has been tested against a TypeSafe account. Replace angle-bracket placeholders with serialized Mewt values. Send `unknown` explicitly where a fact is unavailable; do not include irrelevant fields. Keep the code context concise but sufficient to understand behavior.

```json
{
  "model": "jev-latest",
  "state": {
    "mutant": {
      "language": "<language or dialect>",
      "operator": "<mutation operator/slug, if known>",
      "location": "<stable target and source span>",
      "original_code": "<small enclosing source context>",
      "mutated_code": "<same context with this single mutation applied>",
      "description": "<Mewt-provided mutation description, if available>"
    },
    "test_evidence": {
      "coverage": "<covered / uncovered / unknown>",
      "outcome": "<killed / survived / not run / unknown>",
      "failure_kind": "<assertion / exception / timeout / other / unknown>",
      "related_mutants_or_subsumption": "<known evidence, or unknown>"
    }
  },
  "questions": {
    "tcap_like_priority": {
      "type": "score",
      "instructions": "Estimate how likely this mutant, if presented as a test goal, is to elicit a test that advances test completeness by detecting a useful behavior difference. This is a context-based estimate, not a measured TCAP. Use only the mutant and supplied test evidence. Do not infer coverage, killability, equivalence, or mutant relationships when they are unknown. Do not equate code-change size, bug likelihood, or importance with test-completeness advancement. If evidence is insufficient, choose a middle/uncertain assessment rather than inventing evidence.",
      "criteria": [
        "Very unlikely: evidence strongly suggests the mutant cannot or will not elicit a useful detecting test, for example known equivalence or an unhelpful/unreachable change.",
        "Unlikely: a useful test seems possible but unlikely; the change appears weak or redundant as a test goal.",
        "Plausible: the mutant could elicit a useful test, but evidence is limited or mixed.",
        "Likely: the change appears to expose a meaningful behavior distinction that a test could exercise.",
        "Very likely: supplied evidence strongly supports that a test for this mutant would expose an important behavior distinction and advance completeness."
      ]
    },
    "best_supported_kind": {
      "type": "choice",
      "instructions": "Choose the best-supported characterization from the supplied evidence. These labels describe different properties and are not necessarily mutually exclusive in theory; select the one most useful as a primary annotation. Distinguish confirmed evidence from a source-based hypothesis. A code diff alone normally cannot establish dominator, equivalent, or trivial status. Use unclear when the evidence does not support a category.",
      "criteria": {
        "dominator": "Evidence from mutant-set/test relationships identifies this mutant as a dominator (or strong direct evidence supports that status). A locally plausible or important-looking mutation is not enough.",
        "equivalent": "Evidence indicates the mutant is behaviorally equivalent to the original for all relevant inputs. Do not label it equivalent merely because no supplied test kills it; a survivor may simply lack a suitable test.",
        "trivial": "Evidence indicates any test that executes the mutated location immediately detects the mutant, typically through an unavoidable exception or similarly immediate failure. A merely easy-to-kill mutant is not necessarily trivial.",
        "other_or_subsumed": "The mutant may be a useful or redundant test goal, including a subsumed mutant, but the supplied evidence does not establish one of the preceding labels.",
        "unclear": "The available source and test evidence is insufficient, missing, or conflicting."
      }
    }
  }
}
```

Consume the Score's returned value, distribution, and confidence, and the Choice's selected category, probabilities, and confidence. The score can order results; confidence/probabilities can inform display or a review-needed marker. Do not treat confidence as a calibrated probability that the whole workflow is correct. Keep score thresholds configurable/evaluable rather than assuming a universal cutoff.

### Consider separating classification questions

The paper's properties are not a natural exclusive taxonomy: for example, the paper allows a subsumed mutant to have TCAP 1, and a mutant may be trivial while also participating in subsumption relationships. The single Choice above is a convenient first display label, not a complete formal classification. A more faithful design could ask independent Nouls such as “Is there evidence this mutant is equivalent?” and “Is there evidence this mutant is trivial?”, plus a separate Choice for a mutually exclusive review disposition (e.g., prioritize, lower priority, needs evidence). Use separate Nouls only if those independent flags are useful downstream; otherwise the extra answers add API/token cost and risk confusing users. A dominator judgment should generally be omitted unless Mewt supplies mutant-set/subsumption evidence supporting it.

## How the query applies TCAP—and what it leaves out

### Applied

- **The Score targets the right construct:** likelihood that presenting this mutant elicits a test advancing completeness. That preserves the paper's focus on useful incremental test goals, rather than mutation score or generic defect severity.
- **An ordered rubric suits ranking.** TypeSafe Score represents a position on a described scale and returns probabilities for its levels. The application can sort by the returned score while retaining the distribution for uncertainty-aware display.
- **Evidence is explicit.** Coverage, outcomes, and known relationships can improve the judgment, while explicit unknowns discourage invented execution facts.
- **The query avoids fault coupling.** The paper deliberately does not optimize for known-fault detection; a fault-likelihood prompt would change the target construct and introduce the blind spots the paper discusses.

### Not applied literally

- **The score is not the paper's measured TCAP.** The paper estimates TCAP from test-detection matrices and a learned model of static program context. Jev's Score over a human-written rubric is not trained on Mewt's labels or guaranteed to reproduce those values. The five qualitative levels are a product-facing ordinal judgment, not a calibrated 0–1 TCAP probability. Do not relabel the Score's expected value or confidence as empirical TCAP.
- **No dominator-set or test-completeness computation is performed.** Those require relationships among mutants and tests. Per-mutant code context is insufficient. If Mewt later computes sound/approximate subsumption or a dominator graph, those facts can be supplied as evidence; Jev should not replace that computation.
- **No stopping threshold is prescribed.** The paper investigates TCAP thresholds as potential stopping guidance, but project-dependent variance remains. This draft focuses on ordering individual mutants, not declaring a project adequately tested or telling a user to stop.
- **No claim of reduced engineering effort is made.** The paper evaluates simulated work and test-completeness progress. A Mewt integration must be evaluated against real Mewt workflows before claiming comparable gains.

## Other prompt choices and rationale

- **Per-mutant state is compact and structured.** TypeSafe recommends named state fields when context has multiple parts. Separate original and mutated code avoids asking Jev to reconstruct the edit from an ambiguous description. Include only enough enclosing code to understand the expression's role; large files increase exposure and cost without necessarily improving the judgment.
- **Language and dialect are included.** Mewt is multi-language; syntax and operator meaning vary. A prompt that silently assumes Java would be inappropriate.
- **Test facts distinguish “survived” from “equivalent.”** A mutant surviving the current tests does not prove equivalence. The criteria explicitly prevent this common inference. An exception/timeout/compile error may also have different meanings and should not be flattened to “killed” without an explicit Mewt outcome contract.
- **The Choice includes `unclear` and `other_or_subsumed`.** TypeSafe Choice should include a no-match option when no defined label fits. `unclear` prevents forced certainty; `other_or_subsumed` avoids incorrectly equating “not a dominator” with “useless.”
- **No generated explanation is requested.** The value proposition here is structured ranking/classification. Free-form rationales are not a Jev primitive and would add a separate generation step. If users need reasons, Mewt can show the source diff and supplied test evidence alongside the typed judgment, or add a separately evaluated explanation mechanism.
- **Score confidence is not a workflow correctness guarantee.** TypeSafe documents Score confidence as a measure derived from the level distribution. Use it as an uncertainty signal, not as permission to hide mutants or automate destructive actions. Keep human review and original Mewt results accessible.

## Evaluation and rollout recommendations

Before making rankings prominent or using them to filter mutants:

1. Build a representative set of Mewt mutants across supported languages and operators; include equivalent-like, trivial-like, ordinary survivors, killed mutants, and ambiguous cases where labels can be established.
2. Have people assign the intended judgment and compare both category agreement and ranking quality. Evaluate calibration separately if presenting numeric probabilities.
3. Compare the prioritized review order with random and existing Mewt severity ordering. For a claim about TCAP's intended benefit, measure tests elicited and advancement in a defensible completeness proxy—not just agreement with human judgments.
4. Test failure cases: missing context, malformed/unsupported snippets, unknown test status, low confidence, throttling, outages, and changed mutants/test evidence.
5. Track model/rubric version and input identity; re-run when material evidence changes. Make the result visibly heuristic until domain validation supports stronger claims.

A prudent initial product posture is **opt-in annotation and sorting**, with an unfiltered way to see all mutants. Do not suppress mutants or change mutation-test outcomes based on Jev's classification. This delivers a potentially useful review aid while keeping the distinction between model judgment and measured test evidence clear.
