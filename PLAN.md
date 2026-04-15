# Work Plan

1. ✅ **`2b3c4d5e` – Rust mutation test reorg**  
   Stood up dedicated modules for all 17 Rust slugs, migrated the legacy assertions, removed `mutation_tests.rs`, and refreshed the slug inventory documentation.

2. ✅ **`3c4d5e6f` – Go mutation test reorg**  
   Mirrored the Rust layout with per-slug modules, moved slug assertions out of the integration suite, and added a guard to ensure every active Go slug has a dedicated test.

3. ✅ **`4d5e6f70` – JavaScript mutation test reorg**  
   Mirrored the Rust/Go layout with per-slug modules spanning JS/TS/TSX, moved slug assertions out of the integration suite, centralized the helpers, and added coverage guards for all active slugs.

4. ✅ **`5e6f7081` – Solidity mutation test reorg**  
   Broke out per-slug modules (including Solidity-only RCI/RDV), rebuilt the integration helpers and slug guard, and removed the monolithic `mutation_tests.rs` suite.

5. ✅ **`6f708192` – Enforce per-slug module coverage**  
   Added `tests/slug_module_guard.rs` to assert every engine slug has a per-language module, cleaned up the duplicated per-language guards, and backfilled Go's missing `WF` stub.

6. **`865c298b` – Compound assignment follow-up**  
   Confirm remaining checklist items after the reorgs, update progress, and close out any lingering compound-assignment tasks.

7. **`0f119077` – Allow consumers to override version**  
   Extend `run_main` with an optional version parameter so downstream binaries can surface their own version strings.

8. **`b7f67e3e` – Mutant triage API**  
   Design and implement the triage table, CLI commands, and integrations for classifying mutants (duplicate, redundant, priority levels).
