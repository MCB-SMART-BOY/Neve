#!/bin/bash
# Neve + Helix one-shot setup
# Preferred: use `neve setup helix` instead.
set -e

echo "=== Neve Helix Setup ==="

# 1. Build and install tree-sitter grammar
echo "Building tree-sitter-neve grammar..."
cd "$(dirname "$0")/../../tree-sitter-neve"
~/.cargo/bin/tree-sitter build 2>/dev/null || cargo install tree-sitter-cli --quiet && ~/.cargo/bin/tree-sitter build
mkdir -p ~/.config/helix/runtime/grammars/
cp neve.so ~/.config/helix/runtime/grammars/neve.so
echo "  ✓ grammar installed"

# 2. Install all query files
mkdir -p ~/.config/helix/runtime/queries/neve/
cp queries/highlights.scm ~/.config/helix/runtime/queries/neve/
cp queries/locals.scm ~/.config/helix/runtime/queries/neve/
cp queries/indents.scm ~/.config/helix/runtime/queries/neve/
cp queries/textobjects.scm ~/.config/helix/runtime/queries/neve/
cp queries/injections.scm ~/.config/helix/runtime/queries/neve/
cp queries/folds.scm ~/.config/helix/runtime/queries/neve/
echo "  ✓ queries installed (highlights, locals, indents, textobjects, injections, folds)"

# 3. Install language configuration
cp ../editors/helix/languages.toml ~/.config/helix/languages.toml
echo "  ✓ languages.toml installed"

# 4. Verify
echo ""
echo "=== Verification ==="
echo "neve version: $(neve --version)"
echo "Grammar: $(ls -la ~/.config/helix/runtime/grammars/neve.so | awk '{print $5}')"
echo "Query files: $(ls ~/.config/helix/runtime/queries/neve/ | wc -l)"
echo ""
echo "✅ Setup complete. Open with: hx file.neve"
