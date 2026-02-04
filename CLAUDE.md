# Preflight

Local code review tool for AI-generated changes. See .docs/plans/ for design.

## Tech Stack

- Backend: Rust 1.93.0 + Axum
- Frontend: Svelte 5 + TypeScript, Node 24.x
- Bundler: Vite

## Project Structure

- crates/preflight-core/ — Diff engine, review state, data models
- crates/preflight-server/ — Axum HTTP/WS server
- crates/preflight-mcp/ — MCP server implementation
- frontend/ — Svelte 5 SPA

## Commands

Use `just` for common tasks (see `justfile` for all recipes):

- `just check` — Run all checks (same as CI)
- `just rust` — All Rust checks (fmt, clippy, test)
- `just frontend` — All frontend checks (fmt, lint, type check, build)
- `just build` — Full build (frontend + Rust binary)
- `just run` — Build and run the server at http://127.0.0.1:3000
- `just fmt` — Auto-format everything

Individual checks:

- `just rust-fmt` / `just rust-clippy` / `just rust-test`
- `just frontend-fmt` / `just frontend-lint` / `just frontend-check` / `just frontend-build`

## Worktrees

Use `.worktrees/` for isolated feature work:

- `just worktree-add <branch>` — Create a new worktree with a feature branch
- `just worktree-list` — List all worktrees
- `just worktree-remove <branch>` — Remove a worktree and delete its branch
- `just worktree-clean` — Remove all worktrees

Worktree directory: `.worktrees/` (project-local, gitignored).

## Rules

- Before adding any dependency, verify the latest version:
  Rust: `cargo search <crate> | head -1`
  Node: `npm view <package> version`
- No LLM API calls in the codebase
- Single binary distribution target
