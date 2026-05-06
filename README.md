<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p><em>A standalone language for system configuration, reproducible builds, and structured shell automation / 面向系统配置、可复现构建与结构化 shell 自动化的独立语言</em></p>

<p>
  <a href="https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml">
    <img src="https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/MCB-SMART-BOY/neve/releases">
    <img src="https://img.shields.io/github/v/release/MCB-SMART-BOY/neve?include_prereleases&color=blue" alt="Release">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg" alt="License">
  </a>
  <a href="https://aur.archlinux.org/packages/neve-bin">
    <img src="https://img.shields.io/aur/version/neve-bin?color=1793d1&label=AUR" alt="AUR">
  </a>
</p>

<p><strong>Windows</strong> · <strong>Linux</strong> · <strong>macOS</strong></p>

</div>

Neve is a from-scratch language and toolchain aimed at system configuration, package/build workflows, and safer replacement of ad-hoc shell automation.
Neve 不是 Nix 兼容层，也不是“配置 DSL 外包一层脚本”。它是一门独立语言，目标是把系统配置、构建工作流和 shell 级自动化放进同一套更可检查、更结构化的工具链里。

**Quick Links / 快速链接** · [Docs](docs/) · [Install](docs/user/install.md) · [Quickstart](docs/user/quickstart.md) · [Spec](docs/reference/spec.md) · [API](docs/reference/api.md) · [Feature Matrix](docs/project/feature-matrix.md)

## What Neve Is / Neve 是什么

- A standalone programming language, not just a config DSL / 独立编程语言，而不只是配置 DSL
- A typed, structured surface for system configuration and automation / 用类型化、结构化方式表达系统配置和自动化
- A workspace that includes parser, HIR, type checker, evaluator, formatter, LSP, store, fetch, builder, and CLI / 仓库内同时包含 parser、HIR、type checker、evaluator、formatter、LSP、store、fetch、builder 和 CLI
- A project still converging on one canonical semantic pipeline / 仍在持续收敛到单一 canonical 语义管线的项目

## Current Status / 当前状态

- **Canonical pipeline** (`Parser -> HIR -> Typeck -> HIR Eval`) is the primary execution path.
- **Effect system** (G4 closed): `effect` keyword on functions and impl methods; lambda bodies inherit effect context; `neve check` rejects effectful calls by default.
- **Streaming I/O**: `io.execCommandStreaming` (stdin + line callback), `io.execPipelineStreaming` (pipeline callback), `io.readFileLines`.
- **Atomic file ops**: `io.atomicWrite`, `io.atomicWriteAll` (two-phase commit), `io.copy`, `io.move`.
- **Path literals**: `./path` infers as `Path` type — `io.readFilePath(./test)` works directly.
- **Typed runtime objects**: `Path`, `Bytes`, `Command`, `Pipeline`, `Redirect`, `ProcessResult`, `Task<T>` with timeout + process kill.
- **Scripting**: shebang support, `io.args()`, `io.env()`, `io.sleep()`, `io.which()`.
- **REPL**: history persistence, tab completion, human-friendly display, bracket matching.
- LSP (hover, goto-def, references, rename, completion), formatter, and package workflows exist (Unix-oriented).

如果你想看“到底什么是真实支持、什么还只是路线图”，直接看：

- [Feature Matrix](docs/project/feature-matrix.md)
- [Roadmap](docs/project/roadmap.md)
- [Language Roadmap](docs/project/language-roadmap.md)

## Install / 安装

### Prebuilt / 预编译版本

```bash
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh
```

```powershell
# Windows
irm https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.ps1 | iex
```

### From Source / 从源码安装

```bash
git clone https://github.com/MCB-SMART-BOY/Neve.git
cd Neve

# Install into Cargo's bin directory
cargo install --path neve-cli --locked

# Or build only
cargo build --release
```

手动下载 release、平台说明、AUR、二进制缓存和签名参数说明见：
[docs/user/install.md](docs/user/install.md)

## Quick Start / 快速开始

```bash
neve eval "1 + 2 * 3"
neve repl
neve doc quickstart
```

```neve
-- Streaming command output line by line
io.execCommandStreaming(
    io.command("printf", ["hello\nworld\nneve"]),
    fn(line) { () }
)

-- Path literals are Path-type
let config = io.readFilePath(./Cargo.toml);

-- Atomic file write
io.atomicWrite("/tmp/safe.txt", "atomically written");

-- Effect annotation on functions
fn save(path: Path, content: String) effect = io.writeFilePath(path, content);
```

```bash
neve run hello.neve
neve check hello.neve
```

更系统的上手路径：

- [Quickstart](docs/user/quickstart.md)
- [Tutorial](docs/user/tutorial.md)
- [Spec](docs/reference/spec.md)
- [API](docs/reference/api.md)

## CLI Surface / CLI 入口

- `neve eval "<expr>"` evaluates a single expression.
- `neve run <file>` runs a Neve file.
- `neve check <file>` parses, lowers, and type-checks a file/module graph.
- `neve repl` starts the interactive REPL.
- `neve fmt ...` formats files or directories.
- `neve doc <topic>` opens built-in project documentation in the terminal.
- On Unix, `build`, `package`, `update`, `config`, and `store` expose the package/store/config workflows.

## Repository Layout / 仓库结构

- `neve-cli/`: command-line interface
- `crates/`: language pipeline and system crates
- `docs/`: user, reference, contributor, and project-status docs
- `tests/`: integration and end-to-end smoke coverage
- `examples/`: sample programs and bootstrap package examples

## Development / 开发

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

进一步阅读：

- [Contributor Guide](docs/contributor/contributing.md)
- [Architecture](docs/contributor/architecture.md)
- [Onboarding](docs/contributor/onboarding.md)
- [Tests README](tests/README.md)

## License / 许可证

Neve is licensed under the [Mozilla Public License 2.0](LICENSE).
Neve 使用 [Mozilla Public License 2.0](LICENSE) 许可。
