#!/bin/bash
# Build the tree-sitter grammar for the current platform.
# Run from the repository root: ./scripts/build-grammar.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GRAMMAR_DIR="$REPO_ROOT/tree-sitter-neve"

echo "=== Neve Tree-sitter Grammar Builder ==="
echo "Platform: $(uname -s) $(uname -m)"
echo ""

# 1. Check for tree-sitter CLI
TREE_SITTER=""
if command -v tree-sitter &>/dev/null; then
    TREE_SITTER="tree-sitter"
elif [ -x "$HOME/.cargo/bin/tree-sitter" ]; then
    TREE_SITTER="$HOME/.cargo/bin/tree-sitter"
else
    echo "Installing tree-sitter-cli..."
    cargo install tree-sitter-cli --quiet
    TREE_SITTER="$HOME/.cargo/bin/tree-sitter"
fi
echo "  tree-sitter: $($TREE_SITTER --version 2>&1 | head -1)"

# 2. Check for C compiler (needed for tree-sitter)
if command -v cc &>/dev/null; then
    echo "  C compiler: $(cc --version 2>&1 | head -1)"
elif command -v gcc &>/dev/null; then
    echo "  C compiler: $(gcc --version 2>&1 | head -1)"
elif command -v clang &>/dev/null; then
    echo "  C compiler: $(clang --version 2>&1 | head -1)"
else
    echo "  WARNING: No C compiler found. Grammar build may fail."
fi

# 3. Build the grammar
echo ""
echo "Building grammar..."
cd "$GRAMMAR_DIR"
$TREE_SITTER generate 2>&1 | tail -1
$TREE_SITTER build 2>&1

# 4. Verify output
GRAMMAR_SO="$GRAMMAR_DIR/neve.so"
if [ -f "$GRAMMAR_SO" ]; then
    SIZE=$(ls -la "$GRAMMAR_SO" | awk '{print $5}')
    echo ""
    echo "=== Build Successful ==="
    echo "  Output: $GRAMMAR_SO"
    echo "  Size:   $SIZE bytes"
    echo "  Type:   $(file "$GRAMMAR_SO" | cut -d: -f2-)"
else
    echo ""
    echo "=== Build Failed ==="
    echo "  Grammar .so not found at: $GRAMMAR_SO"
    exit 1
fi

# 5. Platform-specific instructions
echo ""
case "$(uname -s)" in
    Linux)
        echo "Install: cp $GRAMMAR_SO ~/.config/helix/runtime/grammars/neve.so"
        echo "Or run:  neve setup helix"
        ;;
    Darwin)
        echo "Install: cp $GRAMMAR_SO ~/.config/helix/runtime/grammars/neve.dylib"
        echo "Or run:  neve setup helix"
        ;;
    *)
        echo "Install manually to Helix runtime/grammars directory"
        ;;
esac
