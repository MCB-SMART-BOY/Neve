#!/usr/bin/env bash
# Compare test counts with the current git HEAD. / 将测试计数与当前 git HEAD 比较。
# A lower working-tree count is a regression. / 工作树计数降低即为回归。
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT"

pass_count=0
fail_count=0

pass_check() {
    printf '[PASS] %s\n' "$*"
    pass_count=$((pass_count + 1))
}

fail_check() {
    printf '[FAIL] %s\n' "$*" >&2
    fail_count=$((fail_count + 1))
}

if ! command -v git >/dev/null 2>&1; then
    printf '[SKIP] git is unavailable; HEAD baseline cannot be read / git 不可用，无法读取 HEAD 基线\n'
    exit 0
fi
if ! git rev-parse --git-dir >/dev/null 2>&1; then
    printf '[SKIP] repository has no git metadata / 仓库没有 git 元数据\n'
    exit 0
fi
if ! git rev-parse --verify HEAD >/dev/null 2>&1; then
    printf '[SKIP] repository has no HEAD commit / 仓库没有 HEAD 提交\n'
    exit 0
fi

for file in tests/end_to_end.rs tests/parser.rs; do
    if [ ! -f "$file" ]; then
        fail_check "working-tree source missing / 工作树源文件缺失: $file"
    elif ! git cat-file -e "HEAD:$file" 2>/dev/null; then
        fail_check "HEAD source missing / HEAD 源文件缺失: $file"
    fi
done

if [ "$fail_count" -gt 0 ]; then
    printf '[FAIL] regression check cannot read both sources / 回归检查无法读取两个源文件\n' >&2
    exit 1
fi

count_worktree_tests() {
    local file="$1"
    grep -c '#\[test\]' "$file" || true
}

count_head_tests() {
    local file="$1"
    git show "HEAD:$file" | grep -c '#\[test\]' || true
}

check_baseline() {
    local label="$1"
    local head_count="$2"
    local worktree_count="$3"

    if [ "$worktree_count" -lt "$head_count" ]; then
        fail_check "$label count decreased / $label 计数减少: HEAD=$head_count worktree=$worktree_count"
    else
        pass_check "$label count is not below HEAD / $label 计数未低于 HEAD: HEAD=$head_count worktree=$worktree_count"
    fi
}

head_e2e=$(count_head_tests tests/end_to_end.rs)
worktree_e2e=$(count_worktree_tests tests/end_to_end.rs)
head_parser=$(count_head_tests tests/parser.rs)
worktree_parser=$(count_worktree_tests tests/parser.rs)

check_baseline "E2E tests / E2E 测试" "$head_e2e" "$worktree_e2e"
check_baseline "parser tests / parser 测试" "$head_parser" "$worktree_parser"

if [ "$fail_count" -gt 0 ]; then
    printf '[FAIL] regression check failed / 回归检查失败: %d failed, %d passed\n' \
        "$fail_count" "$pass_count" >&2
    exit 1
fi

printf '[PASS] regression check complete / 回归检查完成: %d checks\n' "$pass_count"
