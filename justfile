# Run all checks (same as CI)
check: rust frontend

# All Rust checks
rust: rust-fmt rust-clippy rust-test

# Install frontend dependencies
frontend-install:
    cd frontend && npm ci

# All frontend checks
frontend: frontend-install frontend-fmt frontend-lint frontend-check frontend-test frontend-build

# Rust formatting check
rust-fmt:
    cargo fmt -- --check

# Rust linting
rust-clippy:
    cargo clippy --workspace -- -D warnings

# Rust tests
rust-test:
    cargo test --workspace

# Frontend type checking
frontend-check:
    cd frontend && npm run check

# Frontend linting
frontend-lint:
    cd frontend && npm run lint

# Frontend formatting check
frontend-fmt:
    cd frontend && npm run format:check

# Frontend tests
frontend-test:
    cd frontend && npm test

# Frontend build
frontend-build:
    cd frontend && npm run build

# Full build (frontend + Rust binary)
build: frontend-build
    cargo build

# Auto-format everything
fmt:
    cargo fmt
    cd frontend && npm run format

# Build and run the server
run: build
    cargo run -p preflight-server

# Dev server
dev:
    cd frontend && npm run dev

# Create a new worktree with a feature branch based on latest main
worktree-add branch:
    git fetch origin
    git worktree add .worktrees/{{branch}} -b {{branch}} origin/main
    @echo "Worktree created at .worktrees/{{branch}}"

# List all worktrees
worktree-list:
    git worktree list

# Remove a worktree and its branch
worktree-remove branch:
    git worktree remove .worktrees/{{branch}}
    -git branch -d {{branch}}
    @echo "Removed worktree {{branch}}"

# Remove all worktrees
worktree-clean:
    #!/usr/bin/env bash
    for wt in .worktrees/*/; do
        [ -d "$wt" ] || continue
        branch=$(basename "$wt")
        echo "Removing $branch..."
        git worktree remove "$wt" || true
        git branch -d "$branch" 2>/dev/null || true
    done
    @echo "All worktrees cleaned up"

# Run a test scenario against a running server (default: http://127.0.0.1:3000)
run-scenario:
    node scripts/scenario.ts
