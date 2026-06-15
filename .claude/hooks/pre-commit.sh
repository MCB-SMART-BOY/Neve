#!/usr/bin/env bash
# Pre-commit hook — runs formatting, linting, and fast checks
set -euo pipefail

echo "==> Running cargo fmt --check..."
cargo fmt --all -- --check

echo "==> Running cargo check..."
cargo check --workspace

echo "==> Running cargo clippy (warnings as errors)..."
cargo clippy --workspace --all-targets -- -D warnings

echo "==> Running cargo test..."
cargo test --workspace 2>&1 | tail -5

echo "==> Pre-commit checks passed ✅"
