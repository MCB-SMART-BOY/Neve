```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                          NEVE INSTALLATION GUIDE                              ║
║                               安装指南                                         ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  Requirements / Install / Verify / Troubleshoot  │
│  [中文]     #chinese   ──→  系统要求 / 安装 / 验证 / 常见问题               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

<a name="english"></a>

# English

> *Get Neve running. Should take about 2 minutes.*

## System Requirements

- **OS**: Linux, macOS, Windows
- **RAM**: 512 MB
- **Disk**: 100 MB
- **Docker** (optional): Required for sandbox builds on macOS/Windows

## Platform Feature Matrix

| Feature | Linux | macOS | Windows |
|---------|-------|-------|---------|
| Language Core | ✅ | ✅ | ✅ |
| REPL | ✅ | ✅ | ✅ |
| Formatter/LSP | ✅ | ✅ | ✅ |
| Native Sandbox Build | ✅ | ❌ | ❌ |
| Docker Build | ✅ | ✅ | ✅ |
| System Configuration | ✅ | ❌ | ❌ |

## Quick Install

### Pre-built Binary

```bash
# Linux x86_64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz
tar xzf neve-*.tar.gz
sudo mv neve /usr/local/bin/

# Linux aarch64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-aarch64-unknown-linux-gnu.tar.gz

# macOS
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-apple-darwin.tar.gz

# Verify installation
neve --version
```

### Arch Linux

```bash
yay -S neve-git
```

### Build from Source

```bash
# Requires Rust 1.75+
git clone https://github.com/MCB-SMART-BOY/neve.git
cd neve
cargo build --release
sudo cp target/release/neve /usr/local/bin/
```

## Verify Installation

```bash
neve --version          # Check version
neve info --platform    # Check platform capabilities
neve repl               # Start interactive REPL
neve eval "1 + 2"       # Evaluate expression
```

## Troubleshooting

### "Command not found: neve"

Make sure Neve is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"  # Add to ~/.bashrc or ~/.zshrc
```

### Windows SmartScreen Warning

Click "More info" → "Run anyway"

### Linux Linking Errors

```bash
sudo apt install build-essential  # Debian/Ubuntu
sudo dnf install gcc              # Fedora
sudo pacman -S base-devel         # Arch
```

### macOS Security Warning

```bash
xattr -d com.apple.quarantine neve  # Remove quarantine flag
```

## What's Next

- [Quick Start](quickstart.md) — 5 minutes to first code
- [Tutorial](tutorial.md) — Complete walkthrough
- [Spec](spec.md) — Language reference

---

<a name="chinese"></a>

# 中文

> 装个 Neve，两分钟搞定。

## 系统要求

- **操作系统**: Linux、macOS、Windows 都行
- **内存**: 512 MB
- **硬盘**: 100 MB
- **Docker**（可选）: macOS/Windows 上想用沙箱构建的话需要

## 各平台功能支持

| 功能 | Linux | macOS | Windows |
|------|-------|-------|---------|
| 语言核心 | ✅ | ✅ | ✅ |
| 交互式 REPL | ✅ | ✅ | ✅ |
| 格式化/LSP | ✅ | ✅ | ✅ |
| 原生沙箱构建 | ✅ | ❌ | ❌ |
| Docker 构建 | ✅ | ✅ | ✅ |
| 系统配置 | ✅ | ❌ | ❌ |

## 快速安装

### 下载预编译包

```bash
# Linux x86_64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz
tar xzf neve-*.tar.gz
sudo mv neve /usr/local/bin/

# Linux aarch64 (树莓派等)
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-aarch64-unknown-linux-gnu.tar.gz

# macOS
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-apple-darwin.tar.gz

# 验证一下
neve --version
```

### Arch Linux 用户

```bash
yay -S neve-git
```

### 从源码编译

```bash
# 需要 Rust 1.75 以上
git clone https://github.com/MCB-SMART-BOY/neve.git
cd neve
cargo build --release
sudo cp target/release/neve /usr/local/bin/
```

## 验证安装

```bash
neve --version          # 看看版本
neve info --platform    # 看看当前平台支持哪些功能
neve repl               # 启动交互环境玩玩
neve eval "1 + 2"       # 算个表达式
```

## 常见问题

### 提示 "Command not found: neve"

确保 Neve 在 PATH 里：

```bash
export PATH="$HOME/.local/bin:$PATH"  # 加到 ~/.bashrc 或 ~/.zshrc
```

### Windows SmartScreen 弹窗拦截

点「更多信息」→「仍要运行」

### Linux 链接错误

```bash
sudo apt install build-essential  # Debian/Ubuntu
sudo dnf install gcc              # Fedora
sudo pacman -S base-devel         # Arch
```

### macOS 安全警告

```bash
xattr -d com.apple.quarantine neve  # 去掉隔离标记
```

## 接下来

- [快速入门](quickstart.md) — 5 分钟写出第一段代码
- [完整教程](tutorial.md) — 从头到尾带你过一遍
- [语言规范](spec.md) — 想深入了解就看这个

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                        Installed? Good. Now go hack.
═══════════════════════════════════════════════════════════════════════════════
```

</div>
