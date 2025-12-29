# Neve 安装指南

## 系统要求

- **操作系统**: Linux, macOS, Windows
- **内存**: 512 MB
- **磁盘**: 100 MB
- **Docker** (可选): macOS/Windows 沙箱构建需要

## 平台特性

| 特性 | Linux | macOS | Windows |
|------|-------|-------|---------|
| 语言核心 | ✅ | ✅ | ✅ |
| REPL | ✅ | ✅ | ✅ |
| 格式化/LSP | ✅ | ✅ | ✅ |
| 原生沙箱构建 | ✅ | ❌ | ❌ |
| Docker 构建 | ✅ | ✅ | ✅ |
| 系统配置 | ✅ | ❌ | ❌ |

## 快速安装

### 预编译二进制

```bash
# Linux x86_64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz
tar xzf neve-*.tar.gz
sudo mv neve /usr/local/bin/

# macOS
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-apple-darwin.tar.gz

# 验证
neve --version
```

### 从源码构建

```bash
# 需要 Rust 1.75+
git clone https://github.com/MCB-SMART-BOY/neve.git
cd neve
cargo build --release
sudo cp target/release/neve /usr/local/bin/
```

## 验证安装

```bash
neve --version          # 检查版本
neve info --platform    # 检查平台能力
neve repl               # 启动 REPL
neve eval "1 + 2"       # 求值表达式
```

## 常见问题

### "Command not found: neve"

确保 Neve 在 PATH 中：
```bash
export PATH="$HOME/.local/bin:$PATH"  # 添加到 ~/.bashrc
```

### Windows SmartScreen 警告

点击 "更多信息" → "仍要运行"

### Linux 链接错误

```bash
sudo apt install build-essential  # Debian/Ubuntu
sudo dnf install gcc              # Fedora
```

## 下一步

- 阅读 [快速入门](quickstart.md)
- 查看 [完整教程](tutorial.md)
