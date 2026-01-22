---
alwaysApply: true
---

# Agent Guidelines

## Project Skills
This project has Claude Code skills for specialized tasks:
- `add-language-support` - Use when adding support for a new programming language

Invoke skills using the Skill tool when working on related tasks.

## Issue Tracking with `bd`
- `bd list` - see all open issues
- `bd show <issue-id>` - get full details (e.g., `bd show mewt-2`)
- Issue IDs: `mewt-N` format

## Parallel Workflows with `wt`

### When to Use
Use parallel worktrees when the user requests multiple tasks (especially multiple bd issues).

### Worktree Structure
- Main: `~/code/mewt`
- Subtasks: `~/code/mewt.<branch-name>` (e.g., `~/code/mewt.issue-2`)
- All worktrees share `.git` (commits visible across all worktrees)

### Git Workflow
- Agents CAN commit to worktree branches
- Agents CANNOT commit or merge to main (user reviews and merges)
- Pre-commit hooks (`just check`, `just lint`, `just fmt`) auto-run on commit and must pass

### Example Workflow
For "fix issues 2 and 3":

1. **Gather context:** `bd show mewt-2 && bd show mewt-3`

2. **Launch parallel agents** (single message, multiple Task tools with `subagent_type="general-purpose"`):
   ```
   Fix issue mewt-2. Details: [output from bd show mewt-2]

   1. Run: wt switch --create issue-2
   2. Implement the fix
   3. Commit changes
   4. Report completion (do NOT merge)
   ```
   Repeat for issue-3 using branch `issue-3`.

## Development Commands
- `just check` - Fast syntax/type checking (prefer over full build)
- `just build` - Full compilation
- `just fmt` - Format code (run after each batch of changes)
- `just lint` - Run linters (run after each batch of changes and fix warnings)
- `just test` - Run tests
- `just run` - Run tool against some simple examples

## Database Changes
- Do not change schemas in `migrations/` unless explicitly requested
- If migrations required, ask user for confirmation first
- Always run `just reset-db` after schema/SQL changes

## Engineering Guidelines
- Do not write code before stating assumptions.
- Do not claim correctness you haven't verified.
- Do not handle only the happy path.
- Under what conditions does this work?
