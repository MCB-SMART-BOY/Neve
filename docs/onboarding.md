<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Contributor Onboarding</h1>

<p><em>贡献者入门</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong>
</p>

</div>

This guide helps new contributors understand how the Neve codebase is organized and where to start.
本指南帮助新贡献者理解 Neve 代码库的组织方式与入门路径。

---

## Repository Orientation / 仓库概览

Neve is a Cargo workspace. Most functionality lives in `crates/`, and the CLI lives in `neve-cli/`.

Neve 是一个 Cargo workspace。主要功能在 `crates/` 下，CLI 在 `neve-cli/` 下。

Recommended first steps:

推荐的入门步骤：

- Read `docs/architecture.md` for the high-level flow.
- 阅读 `docs/architecture.md` 了解整体流程。
- Pick one module from the map below and read its `src/lib.rs` first.
- 从下面的模块地图中选一个模块，先阅读其 `src/lib.rs`。

---

## Module Map / 模块地图

Each module below includes its role, core types, and a good entry file.

下列每个模块包含职责、核心类型以及建议的入口文件。

| Module | Role (EN) | 角色 (中文) | Entry Files / 入口文件 |
|---|---|---|---|
| `neve-lexer` | Tokenization and spans | 词法分析与位置标注 | `crates/neve-lexer/src/lib.rs` |
| `neve-syntax` | AST definitions used by parser/formatter/LSP | AST 定义，供解析器/格式化器/LSP 使用 | `crates/neve-syntax/src/lib.rs` |
| `neve-parser` | Recursive descent parser with recovery | 递归下降解析器（含错误恢复） | `crates/neve-parser/src/parser.rs` |
| `neve-hir` | Name resolution + HIR lowering | 名称解析与 HIR 降级 | `crates/neve-hir/src/resolve.rs` |
| `neve-typeck` | HM inference + traits | HM 推断与 Trait 系统 | `crates/neve-typeck/src/check.rs` |
| `neve-eval` | Tree-walk evaluator + lazy runtime | 树遍历求值器 + 惰性运行时 | `crates/neve-eval/src/eval.rs` |
| `neve-std` | Standard library modules | 标准库模块 | `crates/neve-std/src/lib.rs` |
| `neve-frontend` | Frontend pipeline (parse → lower → typeck) | 前端管线（解析 → 降级 → 类型检查） | `crates/neve-frontend/src/lib.rs` |
| `neve-diagnostic` | Diagnostics and ariadne rendering | 诊断与 ariadne 渲染 | `crates/neve-diagnostic/src/lib.rs` |
| `neve-fmt` | AST-based formatter | 基于 AST 的格式化器 | `crates/neve-fmt/src/format.rs` |
| `neve-lsp` | LSP server + symbol index | LSP 服务器与符号索引 | `crates/neve-lsp/src/backend.rs` |
| `neve-cli` | Command-line interface | 命令行界面 | `neve-cli/src/main.rs` |
| `neve-store` | Content-addressed storage | 内容寻址存储 | `crates/neve-store/src/lib.rs` |
| `neve-fetch` | Source fetching (URL/git/local) | 源码获取（URL/git/本地） | `crates/neve-fetch/src/lib.rs` |
| `neve-derive` | Derivation model + hashing | Derivation 模型与哈希 | `crates/neve-derive/src/lib.rs` |
| `neve-builder` | Sandbox build execution | 沙箱构建执行 | `crates/neve-builder/src/lib.rs` |
| `neve-config` | System configuration model | 系统配置模型 | `crates/neve-config/src/lib.rs` |
| `neve-common` | Shared spans/utilities | 共享 Span 与工具 | `crates/neve-common/src/lib.rs` |

---

## Module Walkthrough / 模块细读

### neve-lexer / 词法分析
Turns raw source into tokens with spans that every later phase relies on.
将源文本切分为带 Span 的 token，供后续阶段使用。
Key files: `crates/neve-lexer/src/lexer.rs`, `crates/neve-lexer/src/token.rs`.

### neve-syntax / AST 定义
Defines the AST types that the parser, formatter, and LSP all share.
定义解析器、格式化器、LSP 共享的 AST 类型。
Key files: `crates/neve-syntax/src/ast.rs`, `crates/neve-syntax/src/types.rs`.

### neve-parser / 语法解析
Recursive descent parser with recovery so multiple errors can be reported.
递归下降解析器，支持错误恢复以便一次输出多条诊断。
Key files: `crates/neve-parser/src/parser.rs`, `crates/neve-parser/src/recovery.rs`.

### neve-hir / HIR 解析
Resolves names, imports, and lowers AST into HIR with DefId/LocalId.
解析名称与导入，并把 AST 降级为含 DefId/LocalId 的 HIR。
Key files: `crates/neve-hir/src/resolve.rs`, `crates/neve-hir/src/module_loader.rs`.

### neve-typeck / 类型检查
Runs HM inference, trait resolution, and emits rich diagnostics.
执行 HM 推断、Trait 解析，并输出丰富诊断。
Key files: `crates/neve-typeck/src/check.rs`, `crates/neve-typeck/src/traits.rs`.

### neve-eval / 求值器
Tree-walking evaluator with lazy thunks and builtin primitives.
树遍历求值器，带惰性 thunk 与内建原语。
Key files: `crates/neve-eval/src/eval.rs`, `crates/neve-eval/src/value.rs`.

### neve-std / 标准库
Neve standard library modules used by programs and tooling.
Neve 标准库模块，供语言与工具使用。
Key files: `crates/neve-std/src/lib.rs`, `crates/neve-std/src/list.rs`.

### neve-frontend / 前端管线
Single entry for parse → lower → type check to keep tooling consistent.
统一入口完成 解析 → 降级 → 类型检查，保持工具链一致性。
Key files: `crates/neve-frontend/src/lib.rs`.

### neve-diagnostic / 诊断系统
Diagnostic types, codes, and ariadne rendering helpers.
诊断类型、错误码与 ariadne 渲染辅助。
Key files: `crates/neve-diagnostic/src/diagnostic.rs`, `crates/neve-diagnostic/src/codes.rs`.

### neve-fmt / 格式化器
Formats AST into stable, idempotent source output.
将 AST 格式化为稳定且幂等的源码输出。
Key files: `crates/neve-fmt/src/format.rs`, `crates/neve-fmt/src/printer.rs`.

### neve-lsp / 语言服务器
LSP backend, document analysis, symbol index, and semantic tokens.
LSP 后端、文档分析、符号索引与语义 token。
Key files: `crates/neve-lsp/src/backend.rs`, `crates/neve-lsp/src/document.rs`.

### neve-cli / 命令行
CLI entry point and subcommands for build/check/eval/run/fmt.
CLI 入口与 build/check/eval/run/fmt 子命令。
Key files: `neve-cli/src/main.rs`, `neve-cli/src/commands/`.

### neve-store / 存储
Content-addressed store with GC and NAR utilities.
内容寻址存储，包含 GC 与 NAR 工具。
Key files: `crates/neve-store/src/store.rs`, `crates/neve-store/src/gc.rs`.

### neve-fetch / 获取
Fetches sources from URL, git, or local paths with verification.
从 URL、git、本地路径获取源码并校验。
Key files: `crates/neve-fetch/src/url.rs`, `crates/neve-fetch/src/git.rs`.

### neve-derive / Derivation
Derivation model and hashing for build reproducibility.
Derivation 模型与哈希，用于构建可复现性。
Key files: `crates/neve-derive/src/derivation.rs`, `crates/neve-derive/src/hash.rs`.

### neve-builder / 构建器
Build sandbox orchestration and lifecycle hooks.
构建沙箱编排与生命周期钩子。
Key files: `crates/neve-builder/src/lib.rs`, `crates/neve-builder/src/sandbox.rs`.

### neve-config / 系统配置
System configuration modules and generation management.
系统配置模块与代际管理。
Key files: `crates/neve-config/src/module.rs`, `crates/neve-config/src/generation.rs`.

### neve-common / 通用基础
Shared Span, IDs, and small utilities used across crates.
跨 crate 共享的 Span、ID 与基础工具。
Key files: `crates/neve-common/src/span.rs`, `crates/neve-common/src/interner.rs`.

---

## Pipeline Walkthrough / 流程走读

Source text flows through: Lexer → Parser → HIR resolver → Type checker → Evaluator.

源代码依次经过：词法分析 → 语法解析 → HIR 解析 → 类型检查 → 求值器。

Tooling (LSP, CLI) should prefer `neve-frontend` so diagnostics stay consistent.

工具链（LSP、CLI）应优先使用 `neve-frontend`，以保持诊断一致。

---

## Common Tasks / 常见任务

- Add new syntax: update `docs/spec.md` → parser → formatter → LSP tokens.
- 新增语法：先改 `docs/spec.md` → 再改解析器 → 格式化器 → LSP tokens。

- Add new type rules: update `neve-typeck` and add `tests/typeck.rs` cases.
- 新增类型规则：修改 `neve-typeck`，并补 `tests/typeck.rs` 用例。

- Add LSP features: update `neve-lsp` and `tests/lsp.rs`.
- 新增 LSP 功能：修改 `neve-lsp`，并补 `tests/lsp.rs`。

---

## Where To Start / 推荐起点

- If you like parsing, start at `neve-parser` and `neve-syntax`.
- 如果你喜欢解析器，从 `neve-parser` 与 `neve-syntax` 开始。

- If you like type systems, start at `neve-typeck` and its tests.
- 如果你喜欢类型系统，从 `neve-typeck` 及其测试开始。

- If you like tooling, start at `neve-lsp` and `neve-fmt`.
- 如果你喜欢工具链，从 `neve-lsp` 与 `neve-fmt` 开始。
