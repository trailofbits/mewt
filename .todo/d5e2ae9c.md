---
id: "d5e2ae9c"
title: "Add --severity filter to results and print mutants commands"
tags: ["enhancement", "ux", "filtering"]
---

## Problem

Users need to filter mutation results by severity level to focus on high-priority mutations or exclude noise from low-severity mutations. Currently, there's no way to filter by severity without first knowing which mutation types map to each severity level.

## Solution Implemented

Added `--severity` filter to both `results` and `print mutants` commands that accepts comma-separated severity levels (case-insensitive).

### Usage Examples

```bash
# Show only high severity mutations
mewt results --severity high

# Show high and medium severity
mewt results --severity high,medium

# Works with print mutants too
mewt print mutants --severity low,medium

# Compatible with other filters
mewt results --severity high --status uncaught
mewt print mutants --severity medium --format json
```

**Accepted values:** `high`, `medium`, `low` (case-insensitive)

## Implementation Details

Used **Option 1: Filter in Application Layer** as recommended. The approach:
1. Fetch results/mutants from database (with other filters applied)
2. Apply severity filter by looking up each mutant's slug in the registry
3. Filter out mutants that don't match the requested severity levels

### Changes Made

#### 1. CLI Arguments (src/core/cli.rs)
- Added `severity: Option<String>` field to `ResultsArgs`
- Added `severity: Option<String>` field to `PrintMutantsArgs`
- Help text explains comma-separated values

#### 2. Filter Structs
- Added `severity` field to `ResultsFilters` (src/core/cmds/results.rs)
- Added `severity` field to `MutantsFilters` (src/core/cmds/print.rs)

#### 3. Main Shared (src/core/main_shared.rs)
- Updated instantiation of `ResultsFilters` to pass `severity`
- Updated instantiation of `MutantsFilters` to pass `severity`

#### 4. Language Registry (src/core/registry.rs)
- Added `get_mutation(&self, language_name: &str, slug: &str)` method
- Returns `Option<&Mutation>` for looking up mutation by language and slug

#### 5. Mutation Severity Parsing (src/core/types/mutation.rs)
- Added `#[strum(ascii_case_insensitive)]` attribute to `MutationSeverity` enum
- Allows parsing "high", "High", "HIGH" etc. interchangeably

#### 6. Results Command (src/core/cmds/results.rs)
- Added `parse_severities()` helper function to parse comma-separated string
- Updated `get_results_data()` to:
  - Accept and use `registry` parameter (was previously unused)
  - Apply severity filter after database fetch using `results.retain()`
  - Look up mutation severity via registry for each mutant
- **Fixed** `print_table_format()` to use the already-filtered `data` parameter instead of refetching from database (this was causing the severity filter to be bypassed)

#### 7. Print Mutants Command (src/core/cmds/print/mutants.rs)
- Added imports for `FromStr`, `LanguageRegistry`, `MutationSeverity`
- Added `parse_severities()` helper function
- Updated `execute()` signature to accept `registry: &LanguageRegistry`
- Applied severity filtering in both paths:
  - Filtered query path: Apply filter after `get_mutants_filtered()`
  - Legacy path: Apply filter when iterating through mutants
- Updated caller in `print.rs` to pass registry

## Testing

Comprehensive manual testing performed:

1. **Print Mutants Command:**
   - ✅ `--severity high` - Shows only ER mutations
   - ✅ `--severity medium` - Shows only CR, IF, IT mutations
   - ✅ `--severity low` - Shows only AOS, BL, COS mutations
   - ✅ `--severity high,medium` - Shows combined results
   - ✅ Works with `--format ids` and `--format json`

2. **Results Command:**
   - ✅ `--severity high --all` - Shows only high severity outcomes
   - ✅ `--severity medium --all` - Shows only medium severity outcomes
   - ✅ `--severity low --all` - Shows only low severity outcomes
   - ✅ `--severity high,medium` - Shows combined results
   - ✅ Works with `--format ids` and `--format json`
   - ✅ Severity stats correctly calculated for filtered results

3. **Case Insensitivity:**
   - ✅ `--severity HIGH` works same as `--severity high`
   - ✅ `--severity High,MEDIUM` works correctly

4. **Edge Cases:**
   - ✅ Empty results handled gracefully
   - ✅ Unknown mutations filtered out (severity lookup fails)
   - ✅ Compatible with other filters (--target, --line, --mutation-type, etc.)

All pre-commit checks pass (formatting, linting, typos) and existing test suite remains green ✅

## Benefits

1. **Workflow efficiency:** Users can focus on high-severity mutations first
2. **Noise reduction:** Filter out low-severity mutations when not relevant
3. **Better UX:** No need to remember which mutation types map to which severities
4. **Flexible:** Combine multiple severity levels or use with other filters
5. **Future-proof:** Adding new mutations with severity automatically works

## Related

Complements the `--mutation-type` filter (which filters by specific mutation slugs). Users can now choose:
- Filter by specific types: `--mutation-type ER,CR`
- Filter by severity: `--severity high,medium`
- Combine both: `--severity high --mutation-type ER`

## Post-Implementation Refactoring

Based on code review feedback, several improvements were made:

### 1. Created Generic CSV Parser
- **Issue:** `parse_severities()` function was duplicated in two files
- **Solution:** Created `src/core/utils.rs` with generic `parse_csv<T>()` function
- **Benefits:** 
  - Reusable for any comma-separated value parsing
  - Type conversion happens inline using `FromStr` trait
  - Comprehensive test coverage (8 unit tests)
  - Can be used for other CSV parsing needs in the future

### 2. Removed Unused Parameter
- **Issue:** `print_table_format()` had `_store: &SqlStore` parameter that was never used
- **Root Cause:** Function was refactored to use already-filtered data instead of refetching from database
- **Solution:** Removed the parameter entirely from function signature and call site
- **Benefits:** Cleaner function signature, no misleading parameters

### 3. Registry Method Justification
- **Question:** Why add `get_mutation()` when `get_severity_by_slug()` already exists?
- **Answer:** 
  - Existing code pattern: `registry.get_engine(lang)?.get_severity_by_slug(slug)`
  - New pattern: `registry.get_mutation(lang, slug)?` 
  - More convenient and returns full `Mutation` struct (not just severity)
  - Keeps registry as single point of access rather than exposing engine details
  - Other parts of code use engine methods directly because they need more than severity (e.g., mutation application)

### 4. Confirmed Design Decision
- **Database vs Application-Layer Filtering:** Severity is NOT in the database, only in code
- Application-layer filtering is the only option, which is fine for typical result set sizes
- This keeps the database schema simple and mutations fully defined in code

### Code Quality Improvements
- ✅ No code duplication
- ✅ Cleaner function signatures  
- ✅ Generic, reusable utilities
- ✅ Comprehensive test coverage
- ✅ All pre-commit checks pass
