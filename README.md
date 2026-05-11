<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p>A typed language for system automation.</p>

<p>
  <a href="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml">
    <img src="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/MCB-SMART-BOY/Neve/releases">
    <img src="https://img.shields.io/github/v/release/MCB-SMART-BOY/Neve?color=blue" alt="v3.8.0">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MPL%202.0-brightgreen" alt="MPL-2.0">
  </a>
  <a href="https://aur.archlinux.org/packages/neve-bin">
    <img src="https://img.shields.io/aur/version/neve-bin?color=1793d1&label=AUR" alt="AUR">
  </a>
</p>

<p>Linux · macOS · Windows</p>

</div>

---

文档：[快速入门](docs/user/quickstart.md) · [语言规范](docs/reference/spec.md) · [功能矩阵](docs/project/feature-matrix.md) · [路线图](docs/project/language-roadmap.md) · [更新日志](docs/project/changelog.md)

---

## 这是什么

Neve 最初是为了解决一个具体问题：用同一门语言写系统配置、构建脚本和日常运维工具。

它不是什么配置 DSL，也不是 Bash 的 wrapper。parser、type checker、evaluator、LSP、formatter 都是自己写的，在一个仓库里。

类型系统方面，有静态类型检查、trait/impl、泛型、代数数据类型、模式匹配。跟大部分 ML 系语言差不多。

系统方面，进程、管道、重定向、流式 I/O 都是一等公民。路径字面量 `./config.toml` 直接就是 `Path` 类型，不是字符串。

形式上，核心语义用 Lean 4 做了形式化验证——21 条 EffectEval 规则覆盖了全部 I/O 路径，BinOp 的类型规则 12/12 全部有进度证明，5 项安全审计（H-1, H-2, M-1, M-2, M-4）都过了机器检查。

---

## 装一个

```bash
# 预编译
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh

# Arch
paru -S neve-bin

# 源码
git clone https://github.com/MCB-SMART-BOY/Neve.git
cd Neve && cargo install --path neve-cli --locked
```

---

## 跑一下

```neve
-- 路径字面量
let content = io.readFilePath(./Cargo.toml);

-- 管道
let result = io.execPipeline(
    io.pipeline([io.command("echo", ["hello"]), io.command("cat", [])])
);

-- 流式处理，带超时
let outcome = io.execCommandStreamingWithTimeout(
    io.command("journalctl", ["-f"]),
    fn(line) { println(line) },
    5000
);

-- 标了 effect 的函数才能调用 IO
fn save(path: Path, data: String) effect = io.writeFilePath(path, data);

-- 原子写
io.atomicWrite("/etc/config.toml", newConfig);
```

```bash
neve run script.neve     # 跑脚本
neve repl                # REPL
neve check script.neve   # 类型检查
neve fmt file script.neve # 格式化
```

---

## 仓库结构

```
crates/           Rust 代码（parser, typeck, eval, std, fmt, lsp 等 13 个 crate）
formal/           Lean 4 形式化验证（19 个模块）
tests/            集成测试（222 个 E2E）
docs/             文档
examples/         示例代码
```

---

## 开发

```bash
cargo check --workspace
cargo test --workspace
cargo test --test end_to_end
cd formal && lake build
```

---

MPL-2.0
