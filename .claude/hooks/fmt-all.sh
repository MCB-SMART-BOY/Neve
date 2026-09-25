#!/usr/bin/env bash
# Check that every n3v3 example is already canonically formatted.
# 检查所有 n3v3 示例是否已符合规范格式。
#
# This hook never rewrites files: `n3v3 fmt file <path>` only prints the
# formatted text to stdout, so success there proves nothing about formatting.
# 本钩子不写入文件：`n3v3 fmt file <path>` 只把格式化结果写到 stdout，
# 因此用其退出码判定格式是无效的（曾导致恒 PASS）。校验语义用 `fmt check`。
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"

if [ ! -d examples ]; then
    printf '[FAIL] examples directory missing / examples 目录缺失\n' >&2
    exit 1
fi

mapfile -d '' example_files < <(find examples -type f -name '*.n3v3' -print0)
if [ "${#example_files[@]}" -eq 0 ]; then
    printf '[SKIP] no examples/**/*.n3v3 files found / 未找到 examples/**/*.n3v3 文件\n'
    exit 0
fi

if [ -n "${N3V3_BIN:-}" ]; then
    n3v3_command=("$N3V3_BIN")
elif command -v n3v3 >/dev/null 2>&1; then
    n3v3_command=(n3v3)
elif [ -x target/debug/n3v3 ]; then
    n3v3_command=(target/debug/n3v3)
else
    printf '[FAIL] n3v3 executable not found / 未找到 n3v3 可执行文件\n' >&2
    exit 1
fi

failed=0
for file in "${example_files[@]}"; do
    if "${n3v3_command[@]}" fmt check "$file" >/dev/null 2>&1; then
        printf '[PASS] formatted / 格式合规: %s\n' "$file"
    else
        printf '[FAIL] not canonically formatted / 格式不合规: %s\n' "$file" >&2
        "${n3v3_command[@]}" fmt check "$file" >&2 || true
        failed=$((failed + 1))
    fi
done

if [ "$failed" -gt 0 ]; then
    printf '[FAIL] fmt-all found %d unformatted file(s) / 发现 %d 个格式不合规文件\n' \
        "$failed" "$failed" >&2
    exit 1
fi

printf '[PASS] fmt-all: %d examples are canonically formatted / fmt-all：%d 个示例格式合规\n' \
    "${#example_files[@]}" "${#example_files[@]}"
