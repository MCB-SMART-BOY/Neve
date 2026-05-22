#!/bin/bash
# Neve development environment setup — everything in one command.
# 一键搭建 Neve 开发环境。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "================================================"
echo "  Neve Development Environment Setup"
echo "================================================"
echo ""

# 1. Build the binary
echo "[1/4] Building neve binary..."
cargo build -p neve --release 2>&1 | tail -2
echo ""

# 2. Run tests
echo "[2/4] Running tests..."
cargo test -p neve-lsp --test lsp --test lsp_e2e 2>&1 | tail -5
echo ""

# 3. LSP health check
echo "[3/4] LSP health check..."
./target/release/neve lsp --check 2>&1 | tail -5
echo ""

# 4. Helix setup
echo "[4/4] Editor setup..."
./target/release/neve setup helix 2>&1
echo ""

echo "================================================"
echo "  Setup Complete!"
echo "================================================"
echo ""
echo "  LSP:   neve lsp"
echo "  Check: neve lsp --check"
echo "  Info:  neve lsp --version"
echo "  REPL:  neve repl"
echo "  Helix: hx file.neve"
echo ""
