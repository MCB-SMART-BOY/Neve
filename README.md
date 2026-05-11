<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p>A typed language for system automation — configs, builds, monitoring, scripting. Not a config DSL, not a Bash wrapper.</p>

<p>
  <a href="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml">
    <img src="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/MCB-SMART-BOY/Neve/releases">
    <img src="https://img.shields.io/github/v/release/MCB-SMART-BOY/Neve?color=blue" alt="Release">
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

**Docs**: [Quickstart](docs/user/quickstart.md) · [Spec](docs/reference/spec.md) · [API](docs/reference/api.md) · [Feature Matrix](docs/project/feature-matrix.md) · [Roadmap](docs/project/language-roadmap.md) · [Changelog](docs/project/changelog.md)

---

## 怎么回事

Neve 是一门自己写的语言，给系统运维用。parser、type checker、evaluator、LSP、formatter、包管理器都在一个仓库里。

已经在跑的东西：

- **管道和重定向** — `io.execPipeline`、`io.commandWithRedirects`
- **流式 I/O** — 命令输出逐行处理、文件逐行读，带超时自动杀进程
- **原子写入** — 临时文件 + rename，批量两阶段提交
- **路径字面量** — `./config.toml` 类型是 `Path`，不是 `String`
- **效果系统** — `effect` 关键字标记副作用，`neve check` 默认拒绝 IO
- **超时 + 杀进程** — `io.awaitTaskWithTimeout`，超时后 `kill -9`
- **一等管道** — `cmd1 |> cmd2 |> cmd3` 管道语法（HIR evaluator）
- **REPL** — 历史持久化、Tab 补全、括号匹配、`:type` 查询

形式化验证（`formal/`，19 个 Lean 4 模块）：

- 核心语义的完整形式规范（Syntax, Typing, Eval, Effects）
- 效果系统 21 条 EffectEval 规则，覆盖全部 I/O 路径
- 全部二元运算符的类型安全证明（12/12 BinOp）
- 类型安全定理（`type_safety`）和 `env_preservation` 引理
- 5 项安全审计全部机器检查通过（H-1, H-2, M-1, M-2, M-4）

---

## 装一个

```bash
# 预编译
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh

# Arch
paru -S neve-bin

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

-- 管道
let result = io.execPipeline(
    io.pipeline([io.command("echo", ["hello"]), io.command("cat", [])])
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
