#!/usr/bin/env bash
# Project validator — the single quality-gate entry point.
# 项目校验器 —— 唯一的质量闸门入口。
#
# Every consumer (pre-commit hook, agent workflows, GitHub CI, release checks)
# calls this script instead of re-listing commands, so gates cannot drift apart.
# 所有消费者（pre-commit 钩子、代理工作流、GitHub CI、发布检查）都调用本脚本，
# 不再各自罗列命令，从而避免闸门集合漂移。
#
# Usage / 用法:
#   scripts/validate.sh [--quick|--ci|--release] [--strict] [--list]
#
# Modes / 模式:
#   (default)  所有适用 gate；缺失外部工具 → SKIP
#   --quick    仅快速 gate（format、lint、build、skills、docs），用于 pre-commit
#   --ci       全部 gate 且强制 --strict，用于 CI
#   --release  全部 gate + 文档发布模式检查（版本/changelog）；构建 release CLI，
#              且 cli-smoke 与 docs 两个 gate 使用该 release 二进制
#   --strict   缺失外部工具或前置条件 → FAIL 而不是 SKIP
#   --list     只打印 gate 清单
#
# Gates / 闸门（顺序）:
#   secrets    gitleaks detect --no-git --source .
#   format     cargo fmt --all -- --check
#   lint       cargo clippy --workspace --all-targets -- -D warnings
#   build      cargo check --workspace --all-targets + CLI build (debug; release CLI in --release mode)
#   deps       cargo audit + cargo deny check + trivy fs --scanners vuln,secret,misconfig .
#   test       cargo test --workspace + cargo test --doc --workspace
#   cli-smoke  .claude/skills/run-n3v3/driver.sh
#   docs       scripts/check-docs.sh [--release]
#   skills     .claude/hooks/verify-skills.sh
#
# Exit status / 退出码: 0 = 无 FAIL（且 strict 模式下无 SKIP）；1 = 存在 FAIL。
#
# Gate functions are dispatched indirectly (`gate_${gate//-/_}`), so static
# analysis sees them as unused.
# 闸门函数通过 `gate_${gate//-/_}` 间接分派，静态分析会认为它们未被调用。
# shellcheck disable=SC2329
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

mode=default
strict=0
list_only=0
for arg in "$@"; do
  case "$arg" in
    --quick) mode=quick ;;
    --ci) mode=ci; strict=1 ;;
    --release) mode=release ;;
    --strict) strict=1 ;;
    --list) list_only=1 ;;
    *) printf 'validate.sh: unknown argument: %s\n' "$arg" >&2; exit 2 ;;
  esac
done

ALL_GATES=(secrets format lint build deps test cli-smoke docs skills)
QUICK_GATES=(format lint build skills docs)

gates() {
  if [ "$mode" = quick ]; then
    printf '%s\n' "${QUICK_GATES[@]}"
  else
    printf '%s\n' "${ALL_GATES[@]}"
  fi
}

if [ "$list_only" -eq 1 ]; then
  printf 'mode: %s (strict=%s)\n' "$mode" "$strict"
  gates
  exit 0
fi

failures=0
skipped=0
have() { command -v "$1" >/dev/null 2>&1; }

report() { # status gate
  case "$1" in
    PASS) printf '[PASS] %s\n' "$2" ;;
    FAIL) printf '[FAIL] %s\n' "$2"; failures=$((failures + 1)) ;;
    SKIP) printf '[SKIP] %s\n' "$2"; skipped=$((skipped + 1)) ;;
  esac
  if [ -n "${3:-}" ]; then
    printf '%s\n' "$3" | sed 's/^/       /'
  fi
}

missing_tool() { # gate tool
  if [ "$strict" -eq 1 ]; then
    report FAIL "$1" "required tool '$2' is not installed (strict mode)"
  else
    report SKIP "$1" "tool '$2' is not installed"
  fi
}

run_gate() { # gate command...
  local gate=$1
  shift
  local output
  if output=$("$@" 2>&1); then
    report PASS "$gate"
  else
    report FAIL "$gate" "$(printf '%s\n' "$output" | tail -20)"
  fi
}

gate_secrets() {
  if ! have gitleaks; then missing_tool secrets gitleaks; return; fi
  local config=.gitleaks.toml
  if [ ! -e "$config" ]; then
    report FAIL secrets "missing $config (the scan needs its artifact allowlist)"
    return
  fi
  run_gate secrets gitleaks detect --no-git --source . --redact --config "$config"
}

gate_format() { run_gate format cargo fmt --all -- --check; }

gate_lint() { run_gate lint cargo clippy --workspace --all-targets -- -D warnings; }

build_release_cli() { # release mode: the CLI gates must test current code, not a stale artifact
  cargo check --workspace --all-targets && cargo build --release -p n3v3
}

build_debug_cli() { # the docs gate checks examples/snippets with the debug CLI
  cargo check --workspace --all-targets && cargo build -p n3v3
}

gate_build() {
  if [ "$mode" = release ]; then
    run_gate build build_release_cli
  else
    run_gate build build_debug_cli
  fi
}

gate_deps() {
  local missing=0 tool
  for tool in cargo-audit cargo-deny trivy; do
    if ! have "$tool"; then
      missing_tool deps "$tool"
      missing=1
    fi
  done
  [ "$missing" -eq 1 ] && return

  local output=""
  local status=0 chunk
  for command in "cargo audit" "cargo deny check" "trivy fs --scanners vuln,secret,misconfig ."; do
    # Capture the command's own exit status: `chunk=$(...)` reports it, while
    # appending inside the substitution would report the last command's status.
    # 单独捕获命令退出码：`chunk=$(...)` 的返回值即为该命令状态；若把拼接写进
    # 命令替换里，状态会变成替换内最后一条命令的状态（曾导致本闸门恒 PASS）。
    if ! chunk=$($command 2>&1); then
      status=1
    fi
    output+="$chunk"$'\n'
  done
  if [ "$status" -eq 0 ]; then
    report PASS deps
  else
    report FAIL deps "$(printf '%s\n' "$output" | tail -20)"
  fi
}

gate_test() {
  local test_output doc_output summary result_lines
  if test_output=$(cargo test --workspace 2>&1) && doc_output=$(cargo test --doc --workspace 2>&1); then
    # Report the aggregate, not `tail -5`: the last result lines belong to the
    # smallest test binaries (often `0 passed`), which reads like an empty suite.
    # 报告聚合结果而不是 `tail -5`：最后几行来自最小的测试二进制，常为 `0 passed`，
    # 看起来像没跑测试。
    #
    # `|| true` keeps `pipefail` from killing the script when no result line exists;
    # that case is reported explicitly below instead of silently passing.
    # `|| true` 避免在没有任何结果行时被 `pipefail` 直接终止脚本；这种情况在下面显式报告。
    result_lines=$(printf '%s\n%s\n' "$test_output" "$doc_output" | grep -cE '^test result:' || true)
    if [ "$result_lines" -eq 0 ]; then
      report FAIL test "no 'test result:' line in cargo test output"
      return
    fi
    summary=$(printf '%s\n%s\n' "$test_output" "$doc_output" | grep -E '^test result:' |
      awk '{passed += $4; failed += $6; binaries += 1}
           END {printf "%d passed, %d failed across %d test result line(s)", passed, failed, binaries}' || true)
    report PASS test "$summary"
  else
    report FAIL test "$(printf '%s\n%s\n' "$test_output" "$doc_output" | tail -20)"
  fi
}

gate_cli_smoke() {
  local driver=.claude/skills/run-n3v3/driver.sh
  if [ ! -x "$driver" ]; then
    report FAIL cli-smoke "$driver is missing or not executable"
    return
  fi
  if [ "$mode" = release ]; then
    run_gate cli-smoke "$driver" release
  else
    run_gate cli-smoke "$driver"
  fi
}

gate_docs() {
  local script=scripts/check-docs.sh
  if [ ! -x "$script" ]; then
    report FAIL docs "$script is missing or not executable"
    return
  fi
  if [ "$mode" = release ]; then
    # `--strict` so an internal skip (e.g. a missing CLI binary) fails the gate
    # instead of reporting PASS; point the checker at the binary this mode built.
    # 加 `--strict`：内部跳过（例如 CLI 二进制缺失）必须让闸门失败而不是 PASS；
    # 同时让检查器使用本模式刚构建的二进制。
    N3V3_BIN=${N3V3_BIN:-${CARGO_TARGET_DIR:-./target}/release/n3v3} run_gate docs "$script" --release --strict
  elif [ "$mode" = ci ]; then
    run_gate docs "$script" --strict
  else
    run_gate docs "$script"
  fi
}

gate_skills() {
  local script=.claude/hooks/verify-skills.sh
  if [ ! -x "$script" ]; then
    report FAIL skills "$script is missing or not executable"
    return
  fi
  run_gate skills "$script"
}

while IFS= read -r gate; do
  "gate_${gate//-/_}"
done < <(gates)

printf 'summary: %d failed, %d skipped (%s mode)\n' "$failures" "$skipped" "$mode"
if [ "$failures" -gt 0 ]; then
  exit 1
fi
if [ "$strict" -eq 1 ] && [ "$skipped" -gt 0 ]; then
  printf 'validate.sh: strict mode treats skipped gates as failures\n' >&2
  exit 1
fi
exit 0
