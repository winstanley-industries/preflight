# Run all checks (same as CI)
check: rust frontend

# All Rust checks
rust: rust-fmt rust-clippy rust-test

# Install frontend dependencies
frontend-install:
    cd frontend && npm ci

# All frontend checks
frontend: frontend-install frontend-fmt frontend-lint frontend-check frontend-build

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
