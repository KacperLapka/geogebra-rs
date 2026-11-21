# justfile — build front door for geogebra-rs.
#
# Generated WASM packages (do not edit or commit):
#   frontend/src/lib/pkg_math  <- crates/math_engine
#   frontend/src/lib/pkg_plot  <- crates/plot_engine

set shell := ["bash", "-euo", "pipefail", "-c"]

math_pkg := "frontend/src/lib/pkg_math"
plot_pkg := "frontend/src/lib/pkg_plot"

default:
    @just --list

# One-time local setup.
setup:
    command -v wasm-pack >/dev/null || { echo "wasm-pack not found; install it with: cargo install wasm-pack"; exit 1; }
    rustup target add wasm32-unknown-unknown
    npm --prefix frontend ci

# Build both WASM engines into the paths SvelteKit imports.
wasm:
    rm -rf {{math_pkg}} {{plot_pkg}}
    wasm-pack build crates/math_engine --target web --out-dir ../../{{math_pkg}} --no-gitignore
    wasm-pack build crates/plot_engine --target web --out-dir ../../{{plot_pkg}} --no-gitignore

# Debug WASM builds for a faster Rust edit loop.
wasm-dev:
    rm -rf {{math_pkg}} {{plot_pkg}}
    wasm-pack build crates/math_engine --target web --dev --out-dir ../../{{math_pkg}} --no-gitignore
    wasm-pack build crates/plot_engine --target web --dev --out-dir ../../{{plot_pkg}} --no-gitignore

# Local Postgres from docker-compose.yml.
db:
    docker compose up -d --wait postgres

db-down:
    docker compose down

# API (reads .env; migrations run on startup).
api:
    cargo run -p api

# SvelteKit dev server.
web:
    npm --prefix frontend run dev

# Build WASM, then run the API and Vite together.
dev: wasm-dev
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill $(jobs -p) 2>/dev/null || true' EXIT
    ( cd frontend && npm run dev ) &
    cargo run -p api &
    wait -n

# Format Rust and frontend sources.
fmt:
    cargo fmt --all
    npm --prefix frontend run format

# Fast local checks; no production bundle.
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cargo check -p math_engine -p plot_engine --target wasm32-unknown-unknown
    npm --prefix frontend run check
    npm --prefix frontend run lint

# Production build.
build: wasm
    npm --prefix frontend run build
    cargo build --release -p api

# What CI should run.
ci: check build

# Remove build artifacts.
clean:
    cargo clean
    rm -rf {{math_pkg}} {{plot_pkg}} frontend/.svelte-kit frontend/build frontend/dist

# Show the tools this project expects.
doctor:
    @echo "rustc:     $(rustc --version)"
    @echo "cargo:     $(cargo --version)"
    @echo "wasm-pack: $(wasm-pack --version)"
    @echo "node:      $(node --version)"
    @echo "npm:       $(npm --version)"
    @echo "docker:    $(docker --version)"
