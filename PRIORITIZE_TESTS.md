# Prioritization test record

This records the tests performed during the prioritization refinement described in `PRIORITIZE_RESULT.md`. It distinguishes **real Jev responses** from **local mock responses**. Scores are ordinal heuristics, not calibrated probabilities or TCAP estimates.

At the time of these tests, the compiled post-campaign subcommand was `mewt prioritize survivors`. That is the command recorded below; a separate documentation draft calls it `prioritize results`.

## Archived databases: local-only tests

I surveyed `~/audits/archive/` for SQLite campaign files, then selected five databases covering Rust, Solidity, legacy JavaScript/TypeScript and Sui Move labels, both tested and untested campaigns, and current `Uncaught` results. I opened each source database read-only and used Python's `sqlite3.Connection.backup()` to make a consistent copy under `/tmp/mewt-prioritize-pilot.BHQ7gD/`. All migrations, annotations, and outcome changes described here affected **copies or a new temporary campaign**, never an archived original. No archived source was sent to TypeSafe.

I triggered migration 002 on each copy with `mewt --db /tmp/mewt-prioritize-pilot.BHQ7gD/NAME.sqlite results --id 999999999 --format json`. Each command exited successfully; `_sqlx_migrations` then contained versions 1 and 2. I also ran `results --format json --all` against each copy: the returned result counts matched their outcome counts (including zero for the untested campaign). An initial attempt using `--id -1` was rejected by the CLI as an argument error; the positive nonexistent ID above worked.

The following counts come from a temporary local diagnostic that called `prepare()` on **every saved target** in each migrated copy. A planned mutant is assigned once to a request, not evaluated by Jev in this step. “Pre” includes all mutants, regardless of outcomes; “post” includes only current `Uncaught` outcomes.

| Archive source → copied file | Targets | Mutants | Outcomes | Pre planned (requests) | Post planned (requests) | Unsupported edits |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `60-selini/mewt.sqlite` → `sol.sqlite` | 28 | 1,489 | 1,489; 213 `Uncaught` | 1,489 (201) | 213 (36) | 0 |
| `51-moonpay-usdm/muton.sqlite` → `rust.sqlite` | 2 | 702 | 702; 146 `Uncaught` | 702 (92) | 146 (22) | 0 |
| `61-mina/mewt.sqlite` → `js.sqlite` | 10 | 923 | 923; 187 `Uncaught` | 923 (127) | 187 (34) | 0 |
| `57-delphi-dpm/mewt.sqlite` → `pre.sqlite` | 8 | 1,214 | 0 | 1,214 (156) | 0 (0) | 0 |
| `58-realmarkets/mewt.sqlite` → `move.sqlite` | 33 | 2,655 | 2; both `TestFail` | 2,655 (362) | 0 (0) | 0 |
| **Total** | **81** | **6,983** | | **6,983 (938)** | **546 (92)** | **0** |

Before the window change, two saved JavaScript edits spanning **177 lines** could not fit the 160-line cap (one was also an `Uncaught` survivor). After raising the cap to 192 numbered lines and avoiding full-edit duplication in each question, both fitted within the **16 KiB source** and **32 KiB serialized-request** caps. The largest observed serialized request in this survey was **32,209 bytes** (JavaScript pre); these are locally chosen caps, not verified TypeSafe API limits. Rust, Solidity, JavaScript/TypeScript, and Move planning all completed without unsupported edits.

I also ran a temporary local HTTP mock against **selected saved targets**, not the entire 6,983-mutant archive corpus. The mock returned deliberately synthetic score **2.0** (and `unclear` test focus for post questions); those answers measure request construction, persistence, and display, **not ranking quality**:

| Copied DB and target ID | Pre: evaluated / HTTP requests | Post: evaluated / HTTP requests | Immediate repeat |
| --- | ---: | ---: | --- |
| `sol.sqlite`, target 5 | 9 / 2 | 3 / 1 | All 9 pre and 3 post cached; zero requests |
| `js.sqlite`, target 10 | 4 / 1 | 1 / 1 | All 4 pre and 1 post cached; zero requests |
| `pre.sqlite`, target 7 | 72 / 9 | 0 / 0 | All 72 pre cached; zero requests |
| `move.sqlite`, target 16 | 2 / 1 | 0 / 0 | Both pre cached; zero requests |
| **Total** | **87 / 13** | **4 / 2** | **15 local requests total** |

I ran the temporary diagnostic with `PRIORITIZE_PILOT_DIR=/tmp/mewt-prioritize-pilot.BHQ7gD cargo test --test prioritize_pilot -- --nocapture`. It iterated over every saved target for the planning counts, then sent only the selected targets above to a loopback `TcpListener` through `Client::with_base_url`. The mock checked the request shape and answered each question ID. The copied databases held independent `pre` and `post` rows; `mewt results --id 224 --format json` on `sol.sqlite` and `--id 920` on `js.sqlite` displayed an optional `{score: 2.0, confidence: 1.0, category: "unclear"}` for an `Uncaught` result. **No Rust archive judgments were fetched**, although all Rust edits were planned. The temporary diagnostic test was removed after the pilot; it is not part of the permanent test suite.

## Live TypeSafe check: repository-owned Rust example

I created a new campaign under `/tmp/mewt-prioritize-pilot.BHQ7gD/public/`, with source `fn accepts(value: i32) -> bool { value >= 10 }`. `mewt mutate example.rs` saved five `COS` mutants (`>=` replaced by `==`, `!=`, `<`, `<=`, and `>`). The test command concatenated `example.rs` and a separate `test.inc` containing `#[test] fn threshold_example() { assert!(accepts(10)); }`, then ran `rustc --test combined.rs -o test-bin && ./test-bin`. The separate file let the saved source snapshot remain unchanged. An initial baseline failed because the temporary test text accidentally contained a literal backslash before `!`; I corrected the test fixture and reran successfully. That failure was not a prioritization result.

The main successful workflow, run from that temporary directory, was:

```sh
mewt mutate example.rs
mewt prioritize mutants
mewt run --priority-threshold 2 --comprehensive \
  --test.cmd 'cat example.rs test.inc > combined.rs; rustc --test combined.rs -o test-bin && ./test-bin'
mewt prioritize survivors
mewt results --format json --all
```

With `TYPESAFE_API_KEY` set for the two `prioritize` commands, `mewt prioritize mutants` on the revised rubric sent **one live request** for five mutants. It reported **1,669 input / 79 output tokens** and returned pre-scores:

| Mutant ID | Replacement | Pre-score |
| ---: | --- | ---: |
| 1 | `==` | 3.56 |
| 2 | `!=` | 3.54 |
| 3 | `<` | 3.55 |
| 4 | `<=` | 3.52 |
| 5 | `>` | 3.45 |

A run with `--priority-threshold 2 --comprehensive` and the test command above excluded **zero**, caught **three**, and left **two `Uncaught`**: IDs 1 (`==`) and 4 (`<=`). `mewt prioritize survivors` then sent **one live request** for those two survivors, reporting **1,445 input / 156 output tokens**. Their post-scores and suggested test focuses were **3.31 / `boundary`** (ID 1) and **3.35 / `boundary`** (ID 4). Table and JSON `results` showed the post annotation for eligible `Uncaught` results; caught mutants had none. Repeating both prioritize commands with the key set returned cached results with **zero requests**; reading results and running tests required no key.

To check the **execution policy**, I ran `sqlite3 mewt.sqlite 'DELETE FROM outcomes WHERE mutant_id IN (1,4)'` **only on this temporary database**. Running the saved campaign at `--priority-threshold 4` passed its baseline, excluded both pending mutants, ran zero mutation tests, and wrote **no replacement outcomes**. Running again **without** the threshold tested them and restored their two `Uncaught` outcomes. In a separate check with one outcome removed, `--mutations AOS --priority-threshold 4` filtered out the saved `COS` mutant and reported **zero priority exclusions**; no outcome was written. When a peer's outcome disappeared, the other survivor's shared-batch post annotation stopped appearing until the original survivor set was restored, as required by the conservative cache fingerprint.

**Threshold interpretation:** Since every pre-score was below 4, applying `T=4` *before any mutants had outcomes* would have excluded all five, including the two that later survived. That full five-exclusion run was **not performed**; this is a deduction from the recorded scores and the two-pending-mutant run. Conversely, the observed `T=2` run excluded none. This small example does not justify recommending a threshold or establish whether any score predicts useful tests.

## Automated regression checks

The final implementation ran `just pre-commit`, `just build`, `just test`, and `git diff --check` successfully. The full suite reported **69 library, 3 display, 334 language, and 2 prioritization CLI tests** passing (408 tests total). Among the relevant checks:

- `tests/prioritize.rs`: missing/blank key and privacy notice, invalid and valid threshold arguments, no network/key needed for ordinary results or runs, rejection of an unmatched saved target, and request-window planning against all seven supported language examples.
- `src/core/prioritize.rs` tests: unique bounded windows, oversized-line failure, a 177-line edit, legacy TypeScript/Move dialect hints, localhost mock response attribution, cache hit and partial refresh, preservation after a forced HTTP 401, malformed/mismatched/old-version cache misses, current-`Uncaught`-only post eligibility, and strict threshold behavior (including a `Timeout` row left untouched and re-execution without the flag).
- `src/core/store.rs` and `src/core/cmds/results.rs` tests: migration of an existing campaign, independent pre/post rows, upsert and foreign-key cascade, and optional JSON annotation without changing existing fields. `src/typesafe.rs` tests cover the HTTP endpoint, HTTP failure, question/answer validation, and rejection of a Choice that does not select a highest-probability option.

## Limits of this evidence

**Archived audit source never left the machine**: archived-data evaluations used only a local mock with fabricated answers. Live Jev was exercised only on the small repository-owned Rust example, not on the archived languages or representative production test suites. No developer wrote a new detecting test for these survivors; no production latency, cost, false-exclusion rate, calibration, or cross-language answer quality was measured. The `/tmp/` pilot files and logs are local, temporary evidence, not durable fixtures. No migrations or SQL were changed in this follow-up, and `just reset-db` was not rerun.
