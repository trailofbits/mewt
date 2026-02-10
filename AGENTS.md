---
alwaysApply: true
---

# Agent Guidelines

## Engineering Guidelines
- Do not write code before stating assumptions.
- Do not claim correctness you haven't verified.
- Do not handle only the happy path.
- Under what conditions does this work?

## Development Commands
- Pre-commit checks: `just pre-commit` after every batch of changes (runs fmt-check, check, lint, typos). Fix any reported issues before proceeding.
- Build: `just build`
- Test: `just test`
- Run: `just run <language>` - Run tool against some simple examples in a supported language

## Issue Tracking
- `ls .todo/` - see all task files, or use associated agent extensions
- After resolving a todo, update the associated file with a summary of the solution
- Do not delete todo files until the user has reviewed and confirmed the solution

## Database
- Do not change schemas in `migrations/` unless explicitly requested
- If migrations required, ask user for confirmation first
- Always run `just reset-db` after schema/SQL changes

## Project Skills
This project has Claude Code skills for specialized tasks:
- `add-language-support` - Use when adding support for a new programming language

Invoke skills using the Skill tool when working on related tasks.
