#!/usr/bin/env bash
# Check for test regressions by diffing against last known-good run
set -euo pipefail

echo "==> Checking for test regressions..."
if [ -f /tmp/neve_test_count.txt ]; then
    LAST=$(cat /tmp/neve_test_count.txt)
    NOW=$(cargo test --workspace -- --list 2>/dev/null | wc -l)
    echo "Tests: $LAST → $NOW"
    if [ "$NOW" -lt "$LAST" ]; then
        echo "⚠️  WARNING: Test count decreased! Regression detected."
        exit 1
    fi
fi
echo "==> Regression check passed ✅"
