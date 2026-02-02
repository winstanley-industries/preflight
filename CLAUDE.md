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

- cargo fmt --check / cargo fmt
- cargo clippy -- -D warnings
- cargo test
- cd frontend && npm run check
- cd frontend && npm run lint
- cd frontend && npm run format:check / npm run format
- cd frontend && npm run build

## Rules

- Before adding any dependency, verify the latest version:
  Rust: `cargo search <crate> | head -1`
  Node: `npm view <package> version`
- No LLM API calls in the codebase
- Single binary distribution target
