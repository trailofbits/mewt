---
alwaysApply: true
---

# Agent Guidelines

## Project Skills
This project has Claude Code skills for specialized tasks:
- `add-language-support` - Use when adding support for a new programming language

Invoke skills using the Skill tool when working on related tasks.

## Development Commands
- `just check` - Fast syntax/type checking (prefer over full build)
- `just build` - Full compilation
- `just fmt` - Format code (run after each batch of changes)
- `just lint` - Run linters (run after each batch of changes and fix warnings)
- `just test` - Run tests
- `just run` - Run tool against some simple examples

## Database Changes
- Do not change the database schemas as defined in `migrations/` unless explicitly requested
- If database migrations are required for a given task, halt and ask the user for confirmation
- Always run `just reset-db` after making schema or SQL query changes

## Git Operations
- **ONLY use read-only git commands** - Never modify the working tree
- You can read git history, logs, and status, but do not commit, push, or modify files via git

## Engineering Guidelines
- Do not write code before stating assumptions.
- Do not claim correctness you haven't verified.
- Do not handle only the happy path.
- Under what conditions does this work?
