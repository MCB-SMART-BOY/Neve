<div align="center">

<img src="../../assets/logo.svg" width="120" alt="n3v3 logo">

<h1>Contributor Onboarding</h1>

<p><em>贡献者入门</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

This guide helps new contributors understand how the n3v3 codebase is organized and where to start.
本指南帮助新贡献者理解 n3v3 代码库的组织方式与入门路径。

---

## Repository Orientation / 仓库概览

n3v3 is a Cargo workspace. Most compiler and runtime functionality lives in `crates/`, the CLI lives in `n3v3-cli/`, and editor grammar tooling lives in `tree-sitter-n3v3/`.
n3v3 是一个 Cargo workspace。大多数编译器与运行时功能在 `crates/` 下，CLI 在 `n3v3-cli/` 下，编辑器语法工具在 `tree-sitter-n3v3/` 下。

The workspace declares `edition = "2024"` and therefore requires Rust 1.85 or newer. It does not declare a `rust-version` field.
workspace 声明 `edition = "2024"`，因此要求 Rust 1.85 或更高版本；未声明 `rust-version` 字段。

The workspace-level layout is:
workspace 根目录结构如下：

```text
n3v3/
├── crates/                    # compiler and runtime crates / 编译器与运行时 crate
├── n3v3-cli/                  # CLI binary / CLI 二进制
└── tree-sitter-n3v3/
    ├── grammar.js             # grammar source / 语法源码
    ├── src/parser.c           # committed generated parser / 已提交的生成解析器
    └── bindings/rust/         # Cargo workspace member / Cargo workspace 成员
```

`tree-sitter-n3v3/` provides editor syntax highlighting and parsing. Its Rust binding is a workspace member, but it is not part of the v5 compiler pipeline.
`tree-sitter-n3v3/` 为编辑器提供语法高亮与解析。其 Rust binding 是 workspace 成员，但不参与 v5 编译管线。


Recommended first steps:

推荐的入门步骤：

- Read `docs/contributor/architecture.md` for the high-level flow.
- 阅读 `docs/contributor/architecture.md` 了解整体流程。
- Pick one module from the map below and read its `src/lib.rs` first.
- 从下面的模块地图中选一个模块，先阅读其 `src/lib.rs`。

---

## Module Map / 模块地图

Each module below includes its role, core types, and a good entry file.

下列每个模块包含职责、核心类型以及建议的入口文件。

| Module | Role (EN) | 角色 (中文) | Entry Files / 入口文件 |
|---|---|---|---|
| `n3v3-lexer` | Tokenization and spans | 词法分析与位置标注 | `crates/n3v3-lexer/src/lib.rs` |
| `n3v3-syntax` | AST definitions used by parser/formatter/LSP | AST 定义，供解析器/格式化器/LSP 使用 | `crates/n3v3-syntax/src/lib.rs` |
| `n3v3-parser` | Recursive descent parser with recovery | 递归下降解析器（含错误恢复） | `crates/n3v3-parser/src/parser.rs` |
| `n3v3-hir` | Name resolution + HIR lowering | 名称解析与 HIR 降级 | `crates/n3v3-hir/src/resolve.rs` |
| `n3v3-typeck` | HM inference + traits | HM 推断与 Trait 系统 | `crates/n3v3-typeck/src/check/mod.rs` |
| `n3v3-eval` | Tree-walk evaluator + lazy runtime | 树遍历求值器 + 惰性运行时 | `crates/n3v3-eval/src/eval.rs` (canonical HIR evaluator) |
| `n3v3-std` | Standard library modules | 标准库模块 | `crates/n3v3-std/src/lib.rs` |
| `n3v3-frontend` | Frontend pipeline (parse → lower → typeck) | 前端管线（解析 → 降级 → 类型检查） | `crates/n3v3-frontend/src/lib.rs` |
| `n3v3-diagnostic` | Diagnostics and ariadne rendering | 诊断与 ariadne 渲染 | `crates/n3v3-diagnostic/src/lib.rs` |
| `n3v3-fmt` | AST-based formatter | 基于 AST 的格式化器 | `crates/n3v3-fmt/src/format.rs` |
| `n3v3-lsp` | LSP server + symbol index | LSP 服务器与符号索引 | `crates/n3v3-lsp/src/backend.rs` |
| `n3v3-cli` | Command-line interface | 命令行界面 | `n3v3-cli/src/main.rs` |
| `tree-sitter-n3v3` | Editor syntax highlighting and parsing; outside the v5 compiler pipeline | 编辑器语法高亮与解析；不参与 v5 编译管线 | `tree-sitter-n3v3/grammar.js`, `tree-sitter-n3v3/src/parser.c`, `tree-sitter-n3v3/bindings/rust/` |
| `n3v3-store` | Content-addressed storage | 内容寻址存储 | `crates/n3v3-store/src/lib.rs` |
| `n3v3-fetch` | Source fetching (URL/git/local) | 源码获取（URL/git/本地） | `crates/n3v3-fetch/src/lib.rs` |
| `n3v3-derive` | Derivation model + hashing | Derivation 模型与哈希 | `crates/n3v3-derive/src/lib.rs` |
| `n3v3-builder` | Sandbox build execution | 沙箱构建执行 | `crates/n3v3-builder/src/lib.rs` |
| `n3v3-config` | System configuration model | 系统配置模型 | `crates/n3v3-config/src/lib.rs` |
| `n3v3-common` | Shared spans/utilities | 共享 Span 与工具 | `crates/n3v3-common/src/lib.rs` |

---

## Module Walkthrough / 模块细读

### n3v3-lexer / 词法分析
Turns raw source into tokens with spans that every later phase relies on.
将源文本切分为带 Span 的 token，供后续阶段使用。
Key files: `crates/n3v3-lexer/src/lexer.rs`, `crates/n3v3-lexer/src/token.rs`.

### n3v3-syntax / AST 定义
Defines the AST types that the parser, formatter, and LSP all share.
定义解析器、格式化器、LSP 共享的 AST 类型。
Key files: `crates/n3v3-syntax/src/ast.rs`, `crates/n3v3-syntax/src/types.rs`.

### n3v3-parser / 语法解析
Recursive descent parser with recovery so multiple errors can be reported.
递归下降解析器，支持错误恢复以便一次输出多条诊断。
Key files: `crates/n3v3-parser/src/parser.rs`, `crates/n3v3-parser/src/recovery.rs`.

### n3v3-hir / HIR 解析
Resolves names, imports, and lowers AST into HIR with DefId/LocalId.
解析名称与导入，并把 AST 降级为含 DefId/LocalId 的 HIR。
Key files: `crates/n3v3-hir/src/resolve.rs`, `crates/n3v3-hir/src/module_loader.rs`.

### n3v3-typeck / 类型检查
Runs HM inference, trait resolution, and emits rich diagnostics.
执行 HM 推断、Trait 解析，并输出丰富诊断。
Key files: `crates/n3v3-typeck/src/check/mod.rs`, `crates/n3v3-typeck/src/traits.rs`.

### n3v3-eval / 求值器
Tree-walking evaluator with lazy thunks and builtin primitives.
树遍历求值器，带惰性 thunk 与内建原语。
Key files: `crates/n3v3-eval/src/eval.rs` (canonical HIR evaluator), `crates/n3v3-eval/src/value.rs`.

### n3v3-std / 标准库
n3v3 standard library modules used by programs and tooling.
n3v3 标准库模块，供语言与工具使用。
Key files: `crates/n3v3-std/src/lib.rs`, `crates/n3v3-std/src/list.rs`.

### n3v3-frontend / 前端管线
Single entry for parse → lower → type check to keep tooling consistent.
统一入口完成 解析 → 降级 → 类型检查，保持工具链一致性。
Key files: `crates/n3v3-frontend/src/lib.rs`.

### n3v3-diagnostic / 诊断系统
Diagnostic types, codes, and ariadne rendering helpers.
诊断类型、错误码与 ariadne 渲染辅助。
Key files: `crates/n3v3-diagnostic/src/diagnostic.rs`, `crates/n3v3-diagnostic/src/codes.rs`.

### n3v3-fmt / 格式化器
Formats AST into stable, idempotent source output.
将 AST 格式化为稳定且幂等的源码输出。
Key files: `crates/n3v3-fmt/src/format.rs`, `crates/n3v3-fmt/src/printer.rs`.

### n3v3-lsp / 语言服务器
LSP backend, document analysis, symbol index, and semantic tokens.
LSP 后端、文档分析、符号索引与语义 token。
Key files: `crates/n3v3-lsp/src/backend.rs`, `crates/n3v3-lsp/src/document.rs`.

### n3v3-cli / 命令行
CLI entry point and subcommands for build/check/eval/run/fmt.
CLI 入口与 build/check/eval/run/fmt 子命令。
Key files: `n3v3-cli/src/main.rs`, `n3v3-cli/src/commands/`.

### tree-sitter-n3v3 / 编辑器语法
Maintains the tree-sitter grammar and committed parser output used by editor integrations. It parses source for editor features, but the v5 compiler continues through `n3v3-frontend` and does not consume this parser.
维护编辑器集成使用的 tree-sitter 语法与已提交解析器输出。它为编辑器功能解析源码，但 v5 编译器仍经由 `n3v3-frontend`，不使用该解析器。
Key files: `tree-sitter-n3v3/grammar.js`, `tree-sitter-n3v3/src/parser.c`, `tree-sitter-n3v3/bindings/rust/`.

### n3v3-store / 存储
Content-addressed store with GC and NAR utilities.
内容寻址存储，包含 GC 与 NAR 工具。
Key files: `crates/n3v3-store/src/store.rs`, `crates/n3v3-store/src/gc.rs`.

### n3v3-fetch / 获取
Fetches sources from URL, git, or local paths with verification.
从 URL、git、本地路径获取源码并校验。
Key files: `crates/n3v3-fetch/src/url.rs`, `crates/n3v3-fetch/src/git.rs`.

### n3v3-derive / Derivation
Derivation model and hashing for build reproducibility.
Derivation 模型与哈希，用于构建可复现性。
Key files: `crates/n3v3-derive/src/derivation.rs`, `crates/n3v3-derive/src/hash.rs`.

### n3v3-builder / 构建器
Build sandbox orchestration and lifecycle hooks.
构建沙箱编排与生命周期钩子。
Key files: `crates/n3v3-builder/src/lib.rs`, `crates/n3v3-builder/src/sandbox.rs`.

### n3v3-config / 系统配置
System configuration modules and generation management.
系统配置模块与代际管理。
Key files: `crates/n3v3-config/src/module.rs`, `crates/n3v3-config/src/generation.rs`.

### n3v3-common / 通用基础
Shared Span, IDs, and small utilities used across crates.
跨 crate 共享的 Span、ID 与基础工具。
Key files: `crates/n3v3-common/src/span.rs`, `crates/n3v3-common/src/interner.rs`.

---

## Pipeline Walkthrough / 流程走读

Source text flows through: Lexer → Parser → HIR resolver → Type checker → Evaluator.

源代码依次经过：词法分析 → 语法解析 → HIR 解析 → 类型检查 → 求值器。

Tooling (LSP, CLI) should prefer `n3v3-frontend` so diagnostics stay consistent.

工具链（LSP、CLI）应优先使用 `n3v3-frontend`，以保持诊断一致。

---

## Common Tasks / 常见任务

- Add new syntax: update `docs/reference/spec.md` → parser → formatter → LSP tokens.
- 新增语法：先改 `docs/reference/spec.md` → 再改解析器 → 格式化器 → LSP tokens。

- Add new type rules: update `n3v3-typeck` and add `tests/typeck.rs` cases.
- 新增类型规则：修改 `n3v3-typeck`，并补 `tests/typeck.rs` 用例。

- Add LSP features: update `n3v3-lsp` and `tests/lsp.rs`.
- 新增 LSP 功能：修改 `n3v3-lsp`，并补 `tests/lsp.rs`。

---

## Where To Start / 推荐起点

- If you like parsing, start at `n3v3-parser` and `n3v3-syntax`.
- 如果你喜欢解析器，从 `n3v3-parser` 与 `n3v3-syntax` 开始。

- If you like type systems, start at `n3v3-typeck` and its tests.
- 如果你喜欢类型系统，从 `n3v3-typeck` 及其测试开始。

- If you like tooling, start at `n3v3-lsp` and `n3v3-fmt`.
- 如果你喜欢工具链，从 `n3v3-lsp` 与 `n3v3-fmt` 开始。

---

## Verification Commands / 验证命令

Run these commands from the repository root; they are the core checks used by CI and the project quality gate.
从仓库根目录运行以下命令；它们是 CI 与项目质量闸门使用的核心检查。

```bash
cargo build -p n3v3
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/validate.sh
```
