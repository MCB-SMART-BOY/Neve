#!/usr/bin/env bash
# Format all Neve source files in the examples/ directory
set -euo pipefail

for f in examples/*.neve; do
    if [ -f "$f" ]; then
        echo "Formatting $f..."
        cargo run -p neve -- fmt "$f" 2>/dev/null || echo "  (skipped — fmt not available)"
    fi
done
echo "==> Fmt all done ✅"
