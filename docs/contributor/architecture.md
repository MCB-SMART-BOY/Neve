<div align="center">

<img src="../../assets/logo.svg" width="120" alt="n3v3 logo">

<h1>n3v3 Architecture</h1>

<p><em>架构设计</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

This document gives a high-level architecture overview. For detailed developer references organized by key domains, see the [skill index](../../.claude/skills/README.md).
This index has 13 skill entry files: 12 top-level entries plus `.claude/skills/run-n3v3/SKILL.md`. It is organized by key domains rather than one skill per crate; the workspace contains 17 `crates/*` packages, `n3v3-cli`, and the `tree-sitter-n3v3/bindings/rust/` member.

本文档提供高层架构概览。详细的开发者参考按关键领域维护，请见[技能索引](../../.claude/skills/README.md)。
该索引共有 13 个技能入口：12 个顶层入口，加上 `.claude/skills/run-n3v3/SKILL.md`。它按关键领域组织，而不是每个 crate 一个；workspace 包含 17 个 `crates/*` 包、`n3v3-cli` 以及 `tree-sitter-n3v3/bindings/rust/` 成员。

The workspace declares `edition = "2024"` and therefore requires Rust 1.85 or newer. It does not declare a `rust-version` field.
workspace 声明 `edition = "2024"`，因此要求 Rust 1.85 或更高版本；未声明 `rust-version` 字段。

## Repository Structure and Module Map / 仓库结构与模块地图

The repository-level map is:
仓库级结构如下：

```text
n3v3/
├── crates/                       # compiler and runtime crates / 编译器与运行时 crate
├── n3v3-cli/                     # CLI binary / CLI 二进制
└── tree-sitter-n3v3/
    ├── grammar.js                # grammar source / 语法源码
    ├── src/parser.c              # committed generated parser / 已提交的生成解析器
    └── bindings/rust/            # Cargo workspace member / Cargo workspace 成员
```

`tree-sitter-n3v3/` provides editor syntax highlighting and parsing. Only `tree-sitter-n3v3/bindings/rust/` is a Cargo workspace member; the tree-sitter parser does not participate in the v5 compiler pipeline above.
`tree-sitter-n3v3/` 为编辑器提供语法高亮与解析。只有 `tree-sitter-n3v3/bindings/rust/` 是 Cargo workspace 成员；tree-sitter 解析器不参与上面的 v5 编译管线。

| Area | Role (EN) | 角色 (中文) | Entry Paths / 入口路径 |
|---|---|---|---|
| `crates/` | Compiler and runtime modules | 编译器与运行时模块 | `crates/n3v3-frontend/src/lib.rs` |
| `n3v3-cli/` | Command-line entry point | 命令行入口 | `n3v3-cli/src/main.rs` |
| `tree-sitter-n3v3/` | Editor grammar, syntax highlighting, and parsing; outside the v5 compiler pipeline | 编辑器语法、语法高亮与解析；不参与 v5 编译管线 | `tree-sitter-n3v3/grammar.js`, `tree-sitter-n3v3/src/parser.c`, `tree-sitter-n3v3/bindings/rust/` |

## Developer Reference

For architecture, APIs, key files, and integration points, use the domain-oriented skill index:
架构、API、关键文件与集成点按领域维护，请使用技能索引：
- **Pipeline**: `.claude/skills/n3v3-parser.md` `.claude/skills/n3v3-hir.md` `.claude/skills/n3v3-typeck.md` `.claude/skills/n3v3-eval.md`
- **Stdlib**: `.claude/skills/n3v3-std.md`
- **Tooling**: `.claude/skills/n3v3-lsp.md` `.claude/skills/n3v3-test.md`
- **Workflow and CI**: `.claude/skills/n3v3-dev.md`
- **Formal**: `.claude/skills/n3v3-lean.md`
- **Effects**: `.claude/skills/n3v3-effect.md`
- **Full index**: [`.claude/skills/README.md`](../../.claude/skills/README.md)

## Quick Reference / 快速参考

```bash
cargo build -p n3v3
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/validate.sh
```

---

## Overview / 概述

n3v3 is a pure functional language for system configuration. The pipeline:

```
Source → Lexer (logos) → Parser (LL(1)) → HIR → Typeck (HM) → Eval → Value
```

Canonical entry point: `n3v3-frontend` (shared by CLI, LSP, REPL).

## Platform Support

| Feature | Linux (Tier 1) | macOS (Tier 2) | Windows (Tier 3) |
|---------|:---:|:---:|:---:|
| Language Core | ✅ | ✅ | ✅ |
| REPL + LSP + Fmt | ✅ | ✅ | ✅ |
| Native Sandbox | ✅ | ❌ | ❌ |
| System Config | ✅ | ❌ | ❌ |

See `docs/reference/stability.md` for full tier definitions.

