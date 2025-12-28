#!/bin/bash
# Neve 测试脚本 / Neve Test Script
# 在提交前运行测试以确保代码质量
# Run tests before committing to ensure code quality

set -e  # 遇到错误立即退出 / Exit on error

echo "=== Neve 测试流程 / Neve Test Pipeline ==="
echo ""

# 检查 Rust 是否安装
if ! command -v cargo &> /dev/null; then
    echo "错误: cargo 未找到,请先安装 Rust"
    echo "Error: cargo not found, please install Rust first"
    echo "运行: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "✓ Rust 版本 / Rust version:"
rustc --version
cargo --version
echo ""

# 步骤 1: 检查代码编译
echo "步骤 1/3: 检查代码编译 / Step 1/3: Check compilation"
echo "运行: cargo check --workspace"
if cargo check --workspace; then
    echo "✓ 编译检查通过 / Compilation check passed"
else
    echo "✗ 编译失败 / Compilation failed"
    exit 1
fi
echo ""

# 步骤 2: 运行测试
echo "步骤 2/3: 运行测试 / Step 2/3: Run tests"
echo "运行: cargo test --workspace"
if cargo test --workspace; then
    echo "✓ 测试通过 / Tests passed"
else
    echo "✗ 测试失败 / Tests failed"
    exit 1
fi
echo ""

# 步骤 3: 检查代码格式
echo "步骤 3/3: 检查代码格式 / Step 3/3: Check formatting"
echo "运行: cargo fmt --all -- --check"
if cargo fmt --all -- --check; then
    echo "✓ 代码格式正确 / Code formatting is correct"
else
    echo "⚠ 代码格式不符合标准,运行 'cargo fmt --all' 来自动修复"
    echo "⚠ Code formatting issues found, run 'cargo fmt --all' to fix"
    # 不因格式问题退出,只是警告
    # Don't exit on formatting issues, just warn
fi
echo ""

echo "=== ✓ 所有检查通过 / All checks passed ==="
echo ""
echo "可以安全提交代码 / Safe to commit code"
echo ""

# 如果有参数 --clippy,也运行 clippy
if [[ "$1" == "--clippy" ]]; then
    echo "额外检查: 运行 Clippy / Additional check: Running Clippy"
    cargo clippy --workspace -- -D warnings
fi
