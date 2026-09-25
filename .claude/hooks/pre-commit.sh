#!/usr/bin/env bash
# Run the repository quick quality gate. / 运行仓库快速质量门禁。
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"

if [ ! -f Cargo.toml ] || [ ! -d .claude/hooks ]; then
    printf '[FAIL] not at repository root / 当前不在仓库根目录\n' >&2
    exit 1
fi
if [ ! -f scripts/validate.sh ]; then
    printf '[FAIL] scripts/validate.sh missing / 缺少 scripts/validate.sh\n' >&2
    exit 1
fi

if bash scripts/validate.sh --quick; then
    printf '[PASS] pre-commit quick gate / pre-commit 快速门禁通过\n'
else
    status=$?
    printf '[FAIL] pre-commit quick gate failed (exit %d) / 快速门禁失败（退出码 %d）\n' \
        "$status" "$status" >&2
    exit "$status"
fi
