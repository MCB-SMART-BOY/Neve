#!/bin/bash
# Neve + Helix 一键设置
set -e

echo "=== Neve Helix Setup ==="

# 1. 安装 tree-sitter 语法
echo "Building tree-sitter-neve grammar..."
cd "$(dirname "$0")/../../tree-sitter-neve"
~/.cargo/bin/tree-sitter build 2>/dev/null || cargo install tree-sitter-cli --quiet && ~/.cargo/bin/tree-sitter build
mkdir -p ~/.config/helix/runtime/grammars/
cp neve.so ~/.config/helix/runtime/grammars/neve.so
echo "  ✓ grammar installed"

# 2. 安装高亮查询
mkdir -p ~/.config/helix/runtime/queries/neve/
cp queries/highlights.scm ~/.config/helix/runtime/queries/neve/
echo "  ✓ highlights installed"

# 3. 安装语言配置
cp ../helix/languages.toml ~/.config/helix/languages.toml
echo "  ✓ languages.toml installed"

# 4. 验证
echo ""
echo "=== 验证 ==="
echo "neve 版本: $(neve --version)"
echo "LSP 测试: $(echo '{}' | timeout 1 neve lsp 2>&1 | head -1)"
echo "语法文件: $(ls -la ~/.config/helix/runtime/grammars/neve.so | awk '{print $5}')"
echo "查询文件: $(wc -l < ~/.config/helix/runtime/queries/neve/highlights.scm) lines"
echo ""
echo "✅ 设置完成！用 hx 打开 .neve 文件即可"
echo "   hx examples/learning/01_basics.neve"
