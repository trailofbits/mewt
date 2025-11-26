---
alwaysApply: true
---

# Agent Guidelines

## Development Commands
- `just check` - Fast syntax/type checking (prefer over full build)
- `just build` - Full compilation
- `just fmt` - Format code (run after any batch of changes)
- `just lint` - Run linters
- `just test` - Run tests
- `just run` - Run tool against some simple examples

## Database Changes
- Do not change the database schemas as defined in `migrations/`
- If you think database migrations are required, halt and ask the user for confirmation
- Always run `just reset-db` after making schema or SQL query changes

## Git Operations
- **ONLY use read-only git commands** - Never modify the working tree
- You can read git history, logs, and status, but do not commit, push, or modify files via git

## Engineering Guidelines
- Avoid over-engineering. Only make changes that are directly requested or clearly necessary. Keep solutions simple and focused.
- Don't add features, refactor code, or make "improvements" beyond what was asked. A bug fix doesn't need surrounding code cleaned up. A simple feature doesn't need extra configurability.
- Don't add error handling, fallbacks, or validation for scenarios that can't happen. Trust internal code and framework guarantees. Only validate at system boundaries (user input, external APIs). Don't use backwards-compatibility shims when you can just change the code.
- Don't create helpers, utilities, or abstractions for one-time operations. Don't design for hypothetical future requirements. The right amount of complexity is the minimum needed for the current task. Reuse existing abstractions where possible and follow the DRY principle.

## Code Style Requirements
- **Imports**: Group as std, external crates, then local with `use crate::`
- **Error handling**: Use `thiserror` for custom errors, `Result<T, Error>` pattern, prefer the `?` operator with proper error mapping
- **Async**: Use `async/await` with tokio runtime
- **Types**: All types are in `src/types/`
- **Naming**: snake_case for functions/variables, PascalCase for types
- **Graphics**: Never use emoji characters.
- **SQL queries**: Use sqlx with raw SQL queries inside r#"..."# strings inside the src/store.rs file
