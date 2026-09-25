#!/bin/bash
# n3v3 + Helix one-shot setup
# Preferred: use `n3v3 setup helix` instead.
set -e

echo "=== n3v3 Helix Setup ==="

# 1. Build and install tree-sitter grammar
echo "Building tree-sitter-n3v3 grammar..."
cd "$(dirname "$0")/../../tree-sitter-n3v3"
~/.cargo/bin/tree-sitter build 2>/dev/null || cargo install tree-sitter-cli --quiet && ~/.cargo/bin/tree-sitter build
mkdir -p ~/.config/helix/runtime/grammars/
cp n3v3.so ~/.config/helix/runtime/grammars/n3v3.so
echo "  ✓ grammar installed"

# 2. Install all query files
mkdir -p ~/.config/helix/runtime/queries/n3v3/
cp queries/highlights.scm ~/.config/helix/runtime/queries/n3v3/
cp queries/locals.scm ~/.config/helix/runtime/queries/n3v3/
cp queries/indents.scm ~/.config/helix/runtime/queries/n3v3/
cp queries/textobjects.scm ~/.config/helix/runtime/queries/n3v3/
cp queries/injections.scm ~/.config/helix/runtime/queries/n3v3/
cp queries/folds.scm ~/.config/helix/runtime/queries/n3v3/
echo "  ✓ queries installed (highlights, locals, indents, textobjects, injections, folds)"

# 3. Install language configuration
cp ../editors/helix/languages.toml ~/.config/helix/languages.toml
echo "  ✓ languages.toml installed"

# 4. Verify
echo ""
echo "=== Verification ==="
echo "n3v3 version: $(n3v3 --version)"
echo "Grammar: $(ls -la ~/.config/helix/runtime/grammars/n3v3.so | awk '{print $5}')"
echo "Query files: $(ls ~/.config/helix/runtime/queries/n3v3/ | wc -l)"
echo ""
echo "✅ Setup complete. Open with: hx file.n3v3"
