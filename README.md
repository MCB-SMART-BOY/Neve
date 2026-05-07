<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p>A typed language for system automation — configs, builds, monitoring, scripting. Not a config DSL, not a Bash wrapper.</p>

<p>
  <a href="https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml">
    <img src="https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/MCB-SMART-BOY/neve/releases">
    <img src="https://img.shields.io/github/v/release/MCB-SMART-BOY/neve?color=blue" alt="Release">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg" alt="License">
  </a>
  <a href="https://aur.archlinux.org/packages/neve-bin">
    <img src="https://img.shields.io/aur/version/neve-bin?color=1793d1&label=AUR" alt="AUR">
  </a>
</p>

<p><strong>Linux</strong> · <strong>macOS</strong> · <strong>Windows</strong></p>

</div>

**Docs**: [Quickstart](docs/user/quickstart.md) · [API](docs/reference/api.md) · [Spec](docs/reference/spec.md) · [Feature Matrix](docs/project/feature-matrix.md) · [Reactive Design](docs/project/reactive-design.md)

---

## 怎么回事

Neve 是一门自己写的语言，给系统运维用。parser、type checker、evaluator、LSP、formatter、包管理器都在一个仓库里。

已经在跑的东西：

- **管道和重定向** — `io.execPipeline`、`io.commandWithRedirects`
- **流式 I/O** — 命令输出逐行处理、文件逐行读
- **原子写入** — 临时文件 + rename，批量两阶段提交
- **路径字面量** — `./config.toml` 类型是 `Path`，不是 `String`
- **效果系统** — `effect` 关键字标记副作用，`neve check` 默认拒绝 IO
- **超时 + 杀进程** — `io.awaitTaskWithTimeout`，超时后 `kill -9`
- **REPL** — 历史持久化、Tab 补全、括号匹配

正在设计的东西：

- 反应式 & 触发器 & 时序约束（见 [设计文档](docs/project/reactive-design.md)）

---

## 装一个

```bash
# 预编译
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh

# Arch
paru -S neve-bin

# Nix
nix run github:MCB-SMART-BOY/nix-neve

# Homebrew
brew tap MCB-SMART-BOY/neve && brew install neve

# 或者从源码
git clone https://github.com/MCB-SMART-BOY/Neve.git
cd Neve && cargo install --path neve-cli --locked
```

---

## 跑一下

```neve
-- Hello world（不需要 import）
println("Hello, world!");

-- 路径字面量直接是 Path 类型
let content = io.readFilePath(./Cargo.toml);

-- 流式处理命令输出
io.execCommandStreaming(
    io.command("journalctl", ["-f"]),
    fn(line) { println(line) }
);

-- 原子写
io.atomicWrite("/etc/config.toml", newConfig);

-- 效果注解
fn save(path: Path, data: String) effect = io.writeFilePath(path, data);
```

```bash
neve run script.neve     # 跑脚本
neve repl                # 交互式
neve check script.neve   # 类型检查（默认拒绝 IO）
neve fmt file script.neve # 格式化
```

---

## 开发

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

---

## License

MPL-2.0
