# Prioritization implementation result

The opt-in saved-mutant workflow includes `prioritize mutants` for source-only pre-campaign judgments, `run --priority-threshold T` for conservative exclusions, `prioritize survivors` for current `Uncaught` test-goal judgments, and optional post-campaign annotations in table/JSON results. Pre-campaign output now includes score bands and the lowest-scoring examples; post-campaign output includes score bands and the highest-scoring test goals, with a suggested test focus instead of speculative equivalence/triviality labels. IDs and SARIF remain unchanged. See `docs/prioritization.md` for behavior and privacy limits.

## Storage migration

With approval to change `migrations/`, `002_mutant_priorities.sql` adds one `mutant_priorities` row per `(mutant_id, purpose)` to the campaign database. It constrains purpose to `pre` or `post` and cascades deletion when a mutant or its target is removed. `SqlStore` now handles reads and validated-answer upserts on the same pool as targets and outcomes. A failed refresh leaves the old row untouched. Fingerprints and payload validation still reject stale or malformed entries; a missing/unreadable entry cannot exclude a mutant. There is no companion cache connection or table creation in application code.

Existing experimental `<database>.priorities.sqlite` files are **not automatically imported or deleted**. Re-run `prioritize` to create annotations in the main database; inspect or back up the old file before removing it. This avoids silently trusting orphan entries that lack database-enforced foreign keys.

## Difficulties and resolution

- The original implementation used a separate SQLite cache because schema changes required approval. The approved migration replaces it; tests now exercise migration of an existing campaign, simultaneous pre/post rows, upserts, invalid foreign keys and purposes, and cascading deletion.
- `just reset-db` deletes the local database. I made a SQLite backup first, ran the reset and SQLx preparation, then restored the pre-existing campaign and applied migration 002 to it. Its 55 targets, 3,006 mutants, and two outcomes survived. The backup remains at `/tmp/mewt-priority-db.GnuASB/mewt.sqlite`.
- The migration fixture initially used a malformed file hash, so `get_target` rejected it. I changed the fixture to use a valid hash and reran the focused test. No schema change was needed.
- The request caps (192 numbered source lines, 40 overlap lines, 16 KiB source, 32 KiB request, eight mutants, one in-flight request) are experimental, not published TypeSafe limits. Full edits remain in the state, but long edits are not duplicated in every question. Oversized edits fail visibly instead of receiving a fabricated score. The changed rubric and versioned payload make prior annotations cache misses until re-evaluated.

## Verification and uncertainties

The original implementation passed `just reset-db` after migration; no schema or SQL changed in this follow-up, so the reset was not repeated. This follow-up passed `just pre-commit`, `just build`, `just test`, and `git diff --check`. It tested planning and local mock evaluation on **copied** archived campaigns, never uploading audit source; it also tried both commands against repository-owned Rust code with a live TypeSafe key. All surveyed archive edits fit the updated caps, including two 177-line JavaScript edits that failed with the old cap. The live post query identified boundary tests for two surviving comparison mutants. These are workflow checks, not score calibration.

A real multi-language pilot must measure latency, cost, score usefulness, missed signal, and time saved before recommending any threshold. Scores remain ordinal heuristics, not calibrated TCAP. The `jev-latest` alias can change upstream without changing a local fingerprint; use `--force` to refresh. Survivor requests send only `Uncaught` status; changed logs or duration do not invalidate a judgment while that status remains unchanged.
