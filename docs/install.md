<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Installation Guide</h1>

<p><em>安装指南</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong>
</p>

</div>

---

> *Get Neve running. Should take about 2 minutes.*  
> 装个 Neve，两分钟搞定。

## System Requirements / 系统要求

- **OS**: Linux, macOS, Windows / 操作系统：Linux、macOS、Windows
- **RAM**: 512 MB / 内存：512 MB
- **Disk**: 100 MB / 硬盘：100 MB
- **Docker** (optional): Required for sandbox builds on macOS/Windows / Docker（可选）：macOS/Windows 上想用沙箱构建时需要

## Platform Feature Matrix / 各平台功能支持

| Feature / 功能 | Linux | macOS | Windows |
|---------------|-------|-------|---------|
| Language Core / 语言核心 | ✅ | ✅ | ✅ |
| REPL / 交互式 REPL | ✅ | ✅ | ✅ |
| Formatter/LSP / 格式化与 LSP | ✅ | ✅ | ✅ |
| Native Sandbox Build / 原生沙箱构建 | ✅ | ❌ | ❌ |
| Docker Build / Docker 构建 | ✅ | ✅ | ✅ |
| System Configuration / 系统配置 | ✅ | ❌ | ❌ |

## Quick Install / 快速安装

### Pre-built Binary / 预编译包

```bash
# Linux x86_64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz
tar xzf neve-*.tar.gz
sudo mv neve /usr/local/bin/

# Linux aarch64
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-aarch64-unknown-linux-gnu.tar.gz

# macOS
wget https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-apple-darwin.tar.gz

# Verify installation / 验证一下
neve --version
```

### Arch Linux / Arch Linux 用户

```bash
yay -S neve-git
```

### Build from Source / 从源码编译

```bash
# Requires Rust 1.85+ / 需要 Rust 1.85 以上
git clone https://github.com/MCB-SMART-BOY/neve.git
cd neve
cargo build --release
sudo cp target/release/neve /usr/local/bin/
```

## Verify Installation / 验证安装

```bash
neve --version          # Check version / 查看版本
neve info --platform    # Check platform capabilities / 查看平台支持
neve repl               # Start interactive REPL / 启动 REPL
neve eval "1 + 2"       # Evaluate expression / 计算表达式
```

## Troubleshooting / 常见问题

### "Command not found: neve" / 提示找不到命令

Make sure Neve is in your PATH:
确保 Neve 在 PATH 中：

```bash
export PATH="$HOME/.local/bin:$PATH"  # Add to ~/.bashrc or ~/.zshrc
```

### Windows SmartScreen Warning / SmartScreen 弹窗

Click "More info" → "Run anyway"
点「更多信息」→「仍要运行」

### Linux Linking Errors / Linux 链接错误

```bash
sudo apt install build-essential  # Debian/Ubuntu
sudo dnf install gcc              # Fedora
sudo pacman -S base-devel         # Arch
```

### macOS Security Warning / macOS 安全警告

```bash
xattr -d com.apple.quarantine neve  # Remove quarantine flag
```

## What's Next / 接下来

- [Quick Start](quickstart.md) — 5 minutes to first code / 5 分钟写出第一段代码
- [Tutorial](tutorial.md) — Complete walkthrough / 从头到尾带你过一遍
- [Spec](spec.md) — Language reference / 语言规范

---

> Installed? Good. Now go hack. / 装好就开干吧。
