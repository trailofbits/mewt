---
id: "d5e2ae9c"
title: "Add --severity filter to results and print mutants commands"
tags: ["enhancement", "ux", "filtering"]
---

## Problem

Users need to filter mutation results by severity level to focus on high-priority mutations or exclude noise from low-severity mutations. Currently, there's no way to filter by severity without first knowing which mutation types map to each severity level.

## Expected Behavior

Add `--severity` filter that accepts comma-separated severity levels (case-insensitive):

```bash
# Show only high severity mutations
mewt results --severity high

# Show high and medium severity
mewt results --severity high,medium

# Works with print mutants too
mewt print mutants --severity low,medium
```

**Accepted values:** `high`, `medium`, `low` (case-insensitive)

## Implementation Challenges

Severity is **not stored in the database**. The `mutants` table only stores `mutation_slug`, not `severity`. Severity is defined in the `Mutation` struct in the language registry.

### Option 1: Filter in Application Layer (Recommended)
Filter results after fetching from database, using the registry to map slugs to severities.

**Pros:**
- No database schema changes
- Simple implementation
- No migration needed

**Cons:**
- Less efficient (fetch all, then filter)
- Not a problem for typical result set sizes

**Implementation:**
1. Add `--severity` arg to `ResultsArgs` and `PrintMutantsArgs` in `src/core/cli.rs`
2. Parse CSV into `Vec<MutationSeverity>`
3. Pass severity filter through to execute functions (already have registry access)
4. After fetching results from store, filter by looking up `mutation_slug` in registry
5. Use `registry.get_mutation(language, slug)` to get severity, then filter

### Option 2: Add Severity Column to Database
Add `severity TEXT NOT NULL` column to `mutants` table.

**Pros:**
- Efficient SQL filtering
- Consistent with other filters

**Cons:**
- Requires migration
- Data duplication (severity derivable from slug + registry)
- Breaking change for existing databases

### Option 3: Dynamic SQL with Registry
Build SQL `IN` clause with mutation slugs that match requested severities.

**Pros:**
- Efficient filtering at database layer
- No schema changes

**Cons:**
- Complex: need registry access in store layer
- Store would need to know about all mutations to build slug list

## Recommended Approach

**Use Option 1** (application-layer filtering):
1. Add CLI arguments
2. Fetch results from store (existing functions)
3. Filter in command layer using registry to map slugs → severities
4. Return filtered results

## Files to Change

- `src/core/cli.rs` - Add `severity` field to `ResultsArgs` and `PrintMutantsArgs`
- `src/core/cmds/results.rs` - Add severity to `ResultsFilters`, implement filtering logic
- `src/core/cmds/print.rs` - Add severity to `MutantsFilters`
- `src/core/cmds/print/mutants.rs` - Implement filtering logic

## Example Code Snippet

```rust
// In execute_results, after fetching outcomes:
let filtered_outcomes = if let Some(severities) = &filters.severity {
    outcomes.into_iter().filter(|(mutant, target, _outcome)| {
        if let Some(mutation) = registry.get_mutation(&target.language, &mutant.mutation_slug) {
            severities.contains(&mutation.severity)
        } else {
            false // Filter out unknown mutations
        }
    }).collect()
} else {
    outcomes
};
```

## Related

This complements the `--mutation-types` filter (todo c112a4e1). Users can filter by specific types OR by severity level, giving flexibility in how they triage results.
