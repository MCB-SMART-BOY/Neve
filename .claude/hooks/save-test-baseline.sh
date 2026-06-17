#!/usr/bin/env bash
# Populate the regression test baseline for check-regressions.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
count=$(grep -c '#\[test\]' tests/end_to_end.rs)
echo "$count" > /tmp/neve_test_count.txt
echo "Test baseline saved: $count E2E tests"
