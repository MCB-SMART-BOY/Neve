#!/bin/bash
# n3v3 development environment setup — everything in one command.
# 一键搭建 n3v3 开发环境。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

echo "================================================"
echo "  n3v3 Development Environment Setup"
echo "================================================"
echo ""

# 1. Build the binary
echo "[1/4] Building n3v3 binary..."
cargo build -p n3v3 --release 2>&1 | tail -2
echo ""

# 2. Run tests
echo "[2/4] Running tests..."
cargo test -p n3v3-lsp --test lsp --test lsp_e2e 2>&1 | tail -5
echo ""

# 3. LSP health check
echo "[3/4] LSP health check..."
./target/release/n3v3 lsp --check 2>&1 | tail -5
echo ""

# 4. Helix setup
echo "[4/4] Editor setup..."
./target/release/n3v3 setup helix 2>&1
echo ""

echo "================================================"
echo "  Setup Complete!"
echo "================================================"
echo ""
echo "  LSP:   n3v3 lsp"
echo "  Check: n3v3 lsp --check"
echo "  Info:  n3v3 lsp --version"
echo "  REPL:  n3v3 repl"
echo "  Helix: hx file.n3v3"
echo ""
