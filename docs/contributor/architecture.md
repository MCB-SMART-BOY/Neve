<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Architecture</h1>

<p><em>架构设计</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

This document gives a high-level architecture overview. For detailed per-crate
developer references (APIs, types, integration points, key files), see
`.claude/skills/` — one skill per crate, kept in sync with the actual code.

本文档提供高层架构概览。详细的 per-crate 开发者参考（API、类型、集成点、关键文件）
请见 `.claude/skills/` — 每个 crate 一个 skill，与实际代码保持同步。

---

## Overview / 概述

Neve is a pure functional language for system configuration. The pipeline:

```
Source → Lexer (logos) → Parser (LL(1)) → HIR → Typeck (HM) → Eval → Value
```

Canonical entry point: `neve-frontend` (shared by CLI, LSP, REPL).

## Platform Support

| Feature | Linux (Tier 1) | macOS (Tier 2) | Windows (Tier 3) |
|---------|:---:|:---:|:---:|
| Language Core | ✅ | ✅ | ✅ |
| REPL + LSP + Fmt | ✅ | ✅ | ✅ |
| Native Sandbox | ✅ | ❌ | ❌ |
| System Config | ✅ | ❌ | ❌ |

See `docs/reference/stability.md` for full tier definitions.

## Developer Reference

For crate-level architecture, APIs, key files, and integration points:
- **Pipeline**: `.claude/skills/neve-parser.md` `.claude/skills/neve-hir.md` `.claude/skills/neve-typeck.md` `.claude/skills/neve-eval.md` `.claude/skills/neve-frontend.md`
- **Stdlib**: `.claude/skills/neve-std.md`
- **Tooling**: `.claude/skills/neve-lsp.md` `.claude/skills/neve-test.md`
- **Formal**: `.claude/skills/neve-lean.md`
- **Effects**: `.claude/skills/neve-effect.md`
- **Full index**: `.claude/skills/README.md`

## Quick Reference

```bash
cargo build -p neve                    # build
cargo test --workspace                 # test
cargo clippy --workspace --all-targets -- -D warnings  # lint
.claude/skills/run-neve/driver.sh      # smoke test
```
