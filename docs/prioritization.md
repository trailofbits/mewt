# Optional mutant prioritization

Mewt can ask [TypeSafe](https://typesafe.ai/) Jev for **heuristic** judgments about saved mutants. This is opt-in. It never replaces the observed test outcome, and its 0–4 ordinal scores are **not** measured TCAP probabilities or proofs of equivalence, triviality, or subsumption.

## Workflow

```sh
mewt mutate path/to/source
export TYPESAFE_API_KEY=...  # obtain a key at https://typesafe.ai/
mewt prioritize mutants [TARGET ...] [--force]
mewt run --priority-threshold 2 --test.cmd 'your test command'
mewt prioritize survivors [TARGET ...] [--force]
mewt results
mewt results --format json
```

`prioritize mutants` judges the likely **actionable, nonredundant information from executing tests**, based only on source context. It includes saved mutants with or without outcomes. `prioritize survivors` judges whether writing a **new detecting test** for a mutant with a *current* `Uncaught` outcome would advance test completeness. It asks for a tentative category (`trivial`, `equivalent`, `other_or_subsumed`, or `unclear`) as well as a score. It does not infer dominator status. Scores across the two modes are not interchangeable.

Both commands read **saved** targets and mutants; neither generates mutations nor executes tests. With no `TARGET`, they process all saved targets (independent of `[targets].include`). A path, directory, or glob limits selection to saved paths. `--force` re-queries all selected eligible mutants, replacing only successfully validated judgments; a failed request preserves older data and exits nonzero with evaluated/cached/failed counts. An incomplete run can be resumed without `--force`.

**Privacy:** Prioritization sends bounded source windows, language/dialect, mutation locations and edits to TypeSafe. The survivor command also sends the observed `Uncaught` status; it does **not** send test logs, timestamps, durations, filenames, API keys, or any invented coverage/subsumption evidence. Private code may leave your machine. Both modes require a nonblank `TYPESAFE_API_KEY` even when everything is cached or there are no eligible mutants. No other command needs a key or contacts TypeSafe.

## Execution policy

The threshold is optional, belongs on `run`, and applies only to **already saved** mutants; `run TARGET --priority-threshold T` is rejected. A finite `T` between 0 and 4 is required. Only a **fresh, validated pre-campaign score strictly below T** excludes an otherwise eligible mutant; a score equal to T runs. Missing, invalid, stale, unsupported, or unreadable annotations **run**. A priority exclusion does not write or change any outcome, including an existing `Timeout`, and can be tested later by omitting the flag. Normal severity-order and same-line skip optimization still apply to remaining mutants. Campaign output reports this run's priority exclusions, severity skips, and executed tests separately from the existing database-wide status summary. The baseline test still runs for each command group even if its mutants are excluded.

`results` preserves ordering, filters, IDs-only output, and SARIF output. For `Uncaught` results with fresh post annotations, table output adds a heuristic score, distribution-derived confidence, and tentative category. JSON adds optional `test_goal_priority: {score, confidence, category}` to each eligible result, without changing the existing `mutant`, `target`, or `outcome` fields. When absent or stale, this field is omitted. An annotation never appears on a currently caught mutant. Confidence describes the response distribution, **not** a statistical confidence interval or proof of correctness. No TypeSafe call is made by `results`.

## Cache and limits

Migration `002_mutant_priorities.sql` stores annotations in the **campaign database**, in `mutant_priorities`. The table holds one versioned entry per mutant and purpose (`pre` or `post`), with a foreign key that deletes judgments when their mutant or target is deleted. It records a hash of the saved edit, source snapshot, complete shared request state and questions, model alias, and (for survivors) precisely the `Uncaught` evidence sent. It stores neither source nor test output. Changing the threshold does not invalidate entries. Changes to the rubric, source window, batch membership, or relevant outcome status invalidate them. Changing test output or timing while the mutant remains `Uncaught` does not invalidate a judgment, because these fields are **not sent** to TypeSafe. To clear only annotations, delete rows from `mutant_priorities`, not the campaign database.

Older experimental builds stored annotations in `<database>.priorities.sqlite`. This version does not read or migrate that file; run `prioritize` again to populate the campaign database. The old file remains untouched so you can back it up or remove it yourself after confirming the new judgments.

The current **experimental**, locally chosen caps are 160 source lines, 40 context-overlap lines, 16 KiB of source per window, 32 KiB serialized request bytes, and eight mutants per request, with one in-flight request at a time. They are not TypeSafe service limits. Mutants whose edit or source context cannot fit are reported as failed by `prioritize`, never truncated or silently scored zero. Failures of a batch preserve previously saved entries and do not prevent successful batches from being saved. HTTP 429/529 receive bounded retries; 401/422 and other failures are reported without response bodies. Real service latency, cost, answer quality, and threshold effectiveness have **not** been calibrated on Mewt campaigns. The `jev-latest` alias may change upstream without changing the local input hash; use `--force` to refresh it. Measure missed useful signal versus execution time saved before relying on an exclusion threshold.
