#!/usr/bin/env bash
# Install the pinned external gate tools used by `scripts/validate.sh`.
# 安装 `scripts/validate.sh` 使用的外部闸门工具的固定版本。
#
# This script is the single source of the pinned tool versions: both CI
# (`ci.yml`) and release gates (`release.yml`) call it, so the two pipelines
# cannot drift apart. Local use: run it, then add the printed directory to PATH
# (or set GATE_TOOL_DIR to a directory already on PATH).
# 本脚本是工具版本的唯一来源：CI（`ci.yml`）与发布闸门（`release.yml`）都调用它，
# 两条流水线不会各自漂移。本地使用：运行后把打印的目录加入 PATH（或把
# GATE_TOOL_DIR 指向已在 PATH 中的目录）。
#
# Platform: Linux x86_64 (the CI runner platform). Fails closed - any missing tool
# leaves `scripts/validate.sh --strict` red instead of silently skipping a gate.
# 平台：Linux x86_64（CI runner 平台）。失败即失败——缺少工具会让
# `scripts/validate.sh --strict` 报错，而不是静默跳过闸门。
set -euo pipefail

GITLEAKS_VERSION="${GITLEAKS_VERSION:-8.24.2}"
TRIVY_VERSION="${TRIVY_VERSION:-0.59.1}"
CARGO_AUDIT_VERSION="${CARGO_AUDIT_VERSION:-0.22.2}"
CARGO_DENY_VERSION="${CARGO_DENY_VERSION:-0.20.2}"

tool_dir="${GATE_TOOL_DIR:-${RUNNER_TEMP:-/tmp}/n3v3-gate-tools}"
mkdir -p "$tool_dir"

printf 'Installing gate tools into %s / 安装闸门工具到 %s\n' "$tool_dir" "$tool_dir"

curl --proto '=https' --tlsv1.2 -fsSL --retry 3 \
    "https://github.com/gitleaks/gitleaks/releases/download/v${GITLEAKS_VERSION}/gitleaks_${GITLEAKS_VERSION}_linux_x64.tar.gz" \
    -o "$tool_dir/gitleaks.tar.gz"
tar -xzf "$tool_dir/gitleaks.tar.gz" -C "$tool_dir" gitleaks

curl --proto '=https' --tlsv1.2 -fsSL --retry 3 \
    "https://github.com/aquasecurity/trivy/releases/download/v${TRIVY_VERSION}/trivy_${TRIVY_VERSION}_Linux-64bit.tar.gz" \
    -o "$tool_dir/trivy.tar.gz"
tar -xzf "$tool_dir/trivy.tar.gz" -C "$tool_dir" trivy

chmod +x "$tool_dir/gitleaks" "$tool_dir/trivy"
rm -f "$tool_dir/gitleaks.tar.gz" "$tool_dir/trivy.tar.gz"

cargo install cargo-audit --version "$CARGO_AUDIT_VERSION" --locked
cargo install cargo-deny --version "$CARGO_DENY_VERSION" --locked

if [ -n "${GITHUB_PATH:-}" ]; then
    printf '%s\n' "$tool_dir" >>"$GITHUB_PATH"
    printf 'Added %s to GITHUB_PATH / 已加入 GITHUB_PATH\n' "$tool_dir"
else
    printf 'Add to PATH before running validate.sh / 运行 validate.sh 前请加入 PATH:\n'
    # shellcheck disable=SC2016 # `$PATH` is literal guidance text, not shell expansion.
    printf '  export PATH="%s:$PATH"\n' "$tool_dir"
fi

missing=0
for tool in gitleaks trivy; do
    if ! command -v "$tool" >/dev/null 2>&1 && [ ! -x "$tool_dir/$tool" ]; then
        printf '[FAIL] %s is not available after installation / 安装后仍不可用\n' "$tool" >&2
        missing=1
    fi
done
for subcommand in audit deny; do
    if ! cargo "$subcommand" --version >/dev/null 2>&1; then
        printf '[FAIL] cargo %s is not available after installation / 安装后仍不可用\n' \
            "$subcommand" >&2
        missing=1
    fi
done
if [ "$missing" -ne 0 ]; then
    exit 1
fi

printf '[PASS] gate tools installed / 闸门工具安装完成\n'
