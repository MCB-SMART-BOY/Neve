<div align="center">

<img src="../../assets/logo.svg" width="120" alt="n3v3 logo">

<h1>Contributing to n3v3</h1>

<p><em>贡献指南</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

---

Thank you for your interest in contributing to n3v3! This document provides guidelines and instructions for contributing.
感谢您对 n3v3 项目的关注！本文档提供贡献指南和说明。

## Table of Contents / 目录

- [Code of Conduct / 行为准则](#code-of-conduct--行为准则)
- [Getting Started / 开始贡献](#getting-started--开始贡献)
- [Development Setup / 开发环境](#development-setup--开发环境)
- [Project Structure / 项目结构](#project-structure--项目结构)
- [Code Style / 代码风格](#code-style--代码风格)
- [Documentation Standards / 文档标准](#documentation-standards--文档标准)
- [Pull Request Process / PR 流程](#pull-request-process--pr-流程)
- [Reporting Issues / 报告问题](#reporting-issues--报告问题)

---

## Code of Conduct / 行为准则

Be respectful, inclusive, and constructive. We welcome contributors of all skill levels.

请保持尊重、包容和建设性态度。我们欢迎各种技能水平的贡献者。

---

## Getting Started / 开始贡献

### Prerequisites / 前置要求

- **Rust stable** (1.85+) — the workspace uses `edition = "2024"`, which requires
  Rust 1.85 or newer. The workspace does not declare `rust-version` yet, so no
  lower bound is enforced by the manifest.
  **Rust stable**（1.85+）—— workspace 使用 `edition = "2024"`，要求 Rust 1.85 及以上。
  workspace 尚未声明 `rust-version`，因此 manifest 未机械约束最低版本。
- **Git** - Version control
- **Linux/macOS** - For full functionality (Windows supports language features only)

```bash
# Install Rust stable / 安装 Rust stable
rustup toolchain install stable

# Verify installation / 验证安装
rustc --version  # Should show stable 1.85+ or later
```

---

## Development Setup / 开发环境

```bash
# Clone the repository / 克隆仓库
git clone https://github.com/MCB-SMART-BOY/n3v3.git
cd n3v3

# Build the project / 构建项目
cargo build

# Fast local gates before committing / 提交前的快速闸门
scripts/validate.sh --quick

# Full local verification (same entry point CI uses) / 完整本地验证（与 CI 同一入口）
scripts/validate.sh

# Canonical facts used by documentation and checks / 文档与校验使用的规范事实
scripts/counts.sh

# Documentation standard checker / 文档标准校验器
scripts/check-docs.sh

# Run the CLI / 运行 CLI
cargo run -p n3v3 -- --help

# Format code / 格式化代码
cargo fmt

# Run lints (same flags as CI) / 运行 lint（与 CI 相同参数）
cargo clippy --workspace --all-targets -- -D warnings
```

`scripts/validate.sh` is the single quality-gate entry point; every consumer
(pre-commit hook, agent workflows, CI, release checks) calls it instead of
re-listing commands, so gate definitions cannot drift apart. `--list` prints the
gate set, and a missing external tool (`gitleaks`, `trivy`, `cargo-audit`,
`cargo-deny`) is reported as `[SKIP]` unless `--strict` is passed.
`scripts/validate.sh` 是唯一的质量闸门入口；所有消费者（pre-commit 钩子、代理工作流、
CI、发布检查）都调用它而不再各自罗列命令，因此闸门定义不会漂移。`--list` 打印闸门清单；
外部工具缺失（`gitleaks`、`trivy`、`cargo-audit`、`cargo-deny`）时报告 `[SKIP]`，
除非传入 `--strict`。

`--release` runs the same gates plus the release documentation checks (version and
changelog), and it builds the release CLI so the `cli-smoke` and `docs` gates
exercise that binary rather than a stale one (the other modes use the debug
binary; the smoke driver always rebuilds first). `scripts/check-docs.sh` uses
`./target/debug/n3v3` unless `N3V3_BIN` points somewhere else.
`--release` 运行同样的闸门并追加发布文档检查（版本与 changelog），同时构建 release CLI，
使 `cli-smoke` 与 `docs` 两个闸门验证的是该二进制而不是陈旧产物（其他模式使用 debug
二进制；smoke driver 每次先重新构建）。`scripts/check-docs.sh` 默认使用
`./target/debug/n3v3`，可通过 `N3V3_BIN` 指定其他位置。

For low-memory machines, prefer setting `CARGO_BUILD_JOBS=<n>` yourself instead of relying on a repo-wide hard-coded limit.
如果机器内存较小，建议自行设置 `CARGO_BUILD_JOBS=<n>`，而不是依赖仓库级固定并发值。

---

## Project Structure / 项目结构

```
n3v3/
├── n3v3-cli/              # CLI application / CLI 应用
├── tree-sitter-n3v3/      # Editor grammar + bindings / 编辑器语法与绑定
│   ├── grammar.js         # Grammar source / 语法源码
│   ├── src/parser.c       # Generated parser (committed) / 生成的解析器（已提交）
│   └── bindings/rust/     # Workspace member / workspace 成员
├── crates/
│   ├── n3v3-lexer/        # Tokenizer / 词法分析器
│   ├── n3v3-parser/       # Parser / 语法分析器
│   ├── n3v3-syntax/       # AST definitions / AST 定义
│   ├── n3v3-hir/          # High-level IR / 高级中间表示
│   ├── n3v3-typeck/       # Type checker / 类型检查器
│   ├── n3v3-eval/         # Interpreter / 解释器
│   ├── n3v3-std/          # Standard library / 标准库
│   ├── n3v3-store/        # Content-addressed store / 内容寻址存储
│   ├── n3v3-fetch/        # Source fetching / 源码获取
│   ├── n3v3-builder/      # Build system / 构建系统
│   ├── n3v3-config/       # System configuration / 系统配置
│   ├── n3v3-lsp/          # Language server / 语言服务器
│   ├── n3v3-fmt/          # Code formatter / 代码格式化
│   ├── n3v3-diagnostic/   # Error reporting / 错误报告
│   ├── n3v3-common/       # Shared utilities / 共享工具
│   └── n3v3-derive/       # Proc macros / 过程宏
├── docs/                  # Documentation / 文档
├── examples/              # Runnable examples / 可运行示例
├── tests/                 # Integration tests / 集成测试
└── scripts/               # Helper scripts / 辅助脚本
```

### Data Flow / 数据流

```
Source Code → Lexer → Parser → HIR → TypeChecker → Evaluator
    ↓           ↓        ↓       ↓         ↓           ↓
  .n3v3      Tokens    AST     HIR    Typed HIR    Value
```

---

## Code Style / 代码风格

### General Guidelines / 通用指南

1. **Run formatters before committing / 提交前运行格式化**
   ```bash
   cargo fmt
   cargo clippy --workspace -- -D warnings
   ```

2. **Write bilingual comments / 编写双语注释**
   ```rust
   /// Parse an expression.
   /// 解析表达式。
   fn parse_expr(&mut self) -> Result<Expr> { ... }
   ```

3. **Prefer `?` over `unwrap()` / 优先使用 `?` 而非 `unwrap()`**
   ```rust
   // Bad / 不推荐
   let value = map.get("key").unwrap();
   
   // Good / 推荐
   let value = map.get("key").ok_or(Error::KeyNotFound)?;
   ```

4. **Use descriptive names / 使用描述性名称**
   ```rust
   // Bad / 不推荐
   let x = parse(s)?;
   
   // Good / 推荐
   let expression = parse_expression(source_code)?;
   ```

### Commit Messages / 提交信息

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add pattern matching support
fix: resolve infinite loop in module loading
docs: update installation guide
refactor: simplify type unification
test: add unit tests for lexer
chore: update dependencies
```

---

## Documentation Standards / 文档标准

These rules cover every Markdown file in the repository. The checks that
`scripts/check-docs.sh` actually performs are: canonical count claims in the
forms listed in §2, the current version in the files listed in §3, hub index
coverage, page header blocks, resolvable relative links, `n3v3 doc` registry
consistency, changelog structure, and the executable examples in §6. Everything
else — API/behavior prose in §1, non-canonical phrasings of numbers, per-sentence
version attribution of historical numbers, and the conventions in §4, §7 and §8 —
is a review obligation, not a script check.
本节约束仓库内所有 Markdown 文档。`scripts/check-docs.sh` 实际执行的检查是：
§2 列出的规范计数写法、§3 列出的文件中的当前版本、hub 索引覆盖、页面页头、
相对链接可解析、`n3v3 doc` 注册表一致性、变更日志结构，以及 §6 的可执行示例。
其余内容——§1 中关于 API 与行为一致性的叙述、非规范写法的数字、逐句标注版本
归属的历史数字，以及 §4、§7、§8 的约定——属于评审责任，不由脚本判定。

### 1. Single source of truth / 单一事实源

Documentation never owns facts. Each fact has exactly one owner:

|Fact / 事实|Owner / 所有者|Documentation does / 文档做的事|
|---|---|---|
|Product version|`Cargo.toml` `[workspace.package]`|states it, never invents it|
|Test / method / code counts|`scripts/counts.sh`|states the derived value|
|API signatures and arity|`crates/**`|mirrors the signature|
|Behavior and errors|`tests/**`, `crates/**`|describes it, links the check|

If a document and the code disagree, the code wins and the document is wrong.
文档不拥有事实；文档与代码不一致时，以代码为准，改文档。

### 2. Machine-checkable numbers / 可机械校验的数字

Counts may only be written in these canonical forms, and must equal the values
from `scripts/counts.sh`:

- `<N> E2E tests`, `<N> parser tests`
- `<N> diagnostic codes`, `<N> error codes`
- `<N> LSP methods`
- `<N> canonical keywords`
- `<N> Stream<T> APIs`
- `<N> Lean modules`
- the current product version as `vX.Y.Z`

Chinese mirrors use `N 个 E2E 测试`, `N 个诊断码`, `N 个 LSP 方法`, `N 个关键词`,
`N 个 Lean 模块`. Historical numbers are allowed only when the sentence pins the
version they describe (for example `v0.4.0 当时 7 个主题`). Files that are
historical snapshots by design — `docs/project/changelog.md` and
`.claude/audit-report.md` — are exempt from the count check.
历史数字必须写清所属版本；变更日志与审计快照按设计属于历史记录，不受计数校验约束。

### 3. Structure and index / 结构与索引

- Every page under `docs/` except `docs/README.md` starts with the standard
  header block (centered logo, title, subtitle, navigation) and is indexed in
  `docs/README.md`. The checker fails a page without the header block in its
  first 40 lines, and a page that is not indexed.
- `docs/` 下除 `docs/README.md` 外每页都以统一 header block 开头，并在
  `docs/README.md` 有索引项；缺少页头（前 40 行内）或缺少索引项都会导致校验失败。
- Pages served through `n3v3 doc <topic>` must stay registered: the embedded
  paths and the topic list in `n3v3-cli/src/commands/doc.rs` must match the
  `n3v3 doc` help text.

### 4. Bilingual layout / 双语结构

English first, Chinese mirror immediately after, per section. Code, commands,
identifiers, and error text stay untranslated. Keep one canonical copy of a
signature table instead of repeating it per section; duplicated blocks drift.
同一章节英文在前、中文紧随；代码、命令、标识符、报错保持原文；同一契约只保留一份，避免重复块漂移。

### 5. Links / 链接

Relative Markdown links must resolve to existing files. External links are not
checked by CI (offline-safe gates only).
相对链接必须指向存在的文件；外链不在 CI 中检查（闸门保持离线可用）。

### 6. Executable examples / 可执行示例

- ```` ```n3v3 ```` fences are illustrative fragments.
- ```` ```n3v3-check ```` fences must pass `n3v3 check --allow-effects`; the
  checker extracts every such block and runs it.
- All `examples/**/*.n3v3` files must pass `n3v3 check --allow-effects`.
- ```` ```n3v3 ```` 为说明性片段；```` ```n3v3-check ```` 必须是能通过 `n3v3 check`
  的完整示例；`examples/**/*.n3v3` 全部必须通过检查。

### 7. Status wording / 状态表述

Describe delivery state with `Implemented` / `Experimental` / `Planned` plus the
evidence (file, test, or command). Do not use "Phase N" as a description of the
current delivery status, and remove finished items from gap tables instead of
leaving stale entries.
用 `Implemented` / `Experimental` / `Planned` 加证据描述交付状态；不要用 “Phase N”
表示当前状态；已完成事项应从 gap 表移除。

### 8. Change flow / 变更流程

A code or configuration change updates the affected documents, `.claude/skills/**`,
and the changelog in the same change set; `scripts/validate.sh` must pass before
the change is considered done.
代码或配置变更必须在同一变更集中同步受影响的文档、`.claude/skills/**` 与变更日志；
`scripts/validate.sh` 通过后才算完成。

---

## Pull Request Process / PR 流程

1. **Fork and branch / Fork 并创建分支**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make changes / 修改代码**
   - Write tests for new functionality
   - Update documentation and `.claude/skills/**` in the same change set
   - Run `scripts/validate.sh` (full) or `scripts/validate.sh --quick` while iterating
   - Ensure all tests pass

3. **Submit PR / 提交 PR**
   - Describe what changes you made and why
   - Reference any related issues
   - Wait for CI to pass. CI calls the same entry point
     (`scripts/validate.sh --ci`) and adds a platform build/test matrix defined in
     `.github/workflows/ci.yml`; the workflow file is the source of truth for which
     platform runs which subset.

4. **Code review / 代码审查**
   - Address feedback promptly
   - Keep discussions constructive

---

## Reporting Issues / 报告问题

### Bug Reports / Bug 报告

Include:
- n3v3 version (`n3v3 --version`)
- Operating system
- Minimal reproduction steps
- Expected vs actual behavior

### Feature Requests / 功能请求

Include:
- Use case description
- Proposed solution (if any)
- Alternatives considered

---

## License / 许可证

By contributing, you agree that your contributions will be licensed under the MPL-2.0 license.

通过贡献，您同意您的贡献将在 MPL-2.0 许可证下发布。

---

## Questions? / 有问题？

- Open an issue on GitHub
- Check existing issues and discussions

Thank you for contributing! / 感谢您的贡献！
