# Neve Language Development Skills

A comprehensive skill suite for developing the Neve functional programming language (v4.0.2).

## Available Skills

| Skill | Purpose | Crate(s) |
|-------|---------|----------|
| [neve-dev](neve-dev.md) | General workflow, build, test, CI | workspace |
| [neve-parser](neve-parser.md) | Lexer + recursive descent parser | neve-lexer, neve-parser, neve-syntax |
| [neve-typeck](neve-typeck.md) | Hindley-Milner type inference + Traits | neve-typeck |
| [neve-hir](neve-hir.md) | HIR lowering + name resolution | neve-hir |
| [neve-eval](neve-eval.md) | HIR tree-walking interpreter | neve-eval |
| [neve-std](neve-std.md) | Standard library (I/O, Stream<T>, List) | neve-std |
| [neve-lsp](neve-lsp.md) | Language Server Protocol | neve-lsp |
| [neve-test](neve-test.md) | Testing strategy + 541 E2E tests | tests/ |
| [neve-diagnostic](neve-diagnostic.md) | Compiler error system (55 codes) | neve-diagnostic |
| [neve-fmt](neve-fmt.md) | Code formatter | neve-fmt |
| [neve-effect](neve-effect.md) | Effect system (EffectEval v4.3) | neve-typeck, neve-eval |
| [neve-lean](neve-lean.md) | Formal verification (Lean 4) | formal/ |

## Language Pipeline

```
Source Code (.neve)
       │
       ▼
┌──────────────┐  neve-lexer (logos)
│   Lexer      │  Tokens with Span
└──────┬───────┘
       │
       ▼
┌──────────────┐  neve-parser (recursive descent LL(1))
│   Parser     │  AST: Module { items: [...] }
└──────┬───────┘
       │
       ▼
┌──────────────┐  neve-hir (name resolution + lowering)
│  HIR Lower   │  Resolved HIR with scoped definitions
└──────┬───────┘
       │
       ▼
┌──────────────┐  neve-typeck (Hindley-Milner + Traits)
│  Type Check  │  Typed HIR with inferred types
└──────┬───────┘
       │
       ▼
┌──────────────┐  neve-eval (tree-walking, lazy)
│  Evaluator   │  Value: Int | String | List | ...
└──────────────┘
```

## Syntax Quick Reference

| Feature | v3.0 | v4.0 |
|---------|------|------|
| Type definition | `type Foo = { }` / `type Bar = \| ...` | same |
| Import | `use std.list` | same |
| Import alias | `use std.io as io` | `use std.io = io` |
| Lambda | `\|x\| x + 1` | same |
| Record | `{ x = 1 }` | same |
| Comment | `& comment` (also `--` still works) | same |
| Top-level | `let`/`fn`/`;` optional | same |
| Record merge | `a & b` | same |
| If expression | `if cond then a else b` | `if cond -> a else b` |
| Lazy evaluation | `lazy expr` | `~expr` |
| Effect annotation | `fn foo() effect = ...` | `fn foo() = ...` (auto-inferred) |
| Visibility | `pub fn` | `fn` (all public by default) |
| Keywords | 17 | 12 |

## Crate Map

| Crate | Responsibility |
|-------|----------------|
| `neve-lexer` | Tokenization (logos) |
| `neve-parser` | Recursive descent parser (LL(1)) |
| `neve-syntax` | AST node definitions + source file |
| `neve-hir` | High-level IR + name resolution |
| `neve-typeck` | Hindley-Milner type inference + Traits |
| `neve-frontend` | Pipeline facade (driver + session) |
| `neve-eval` | HIR tree-walking interpreter |
| `neve-std` | Standard library (I/O, Stream<T>, collections) |
| `neve-lsp` | Language Server Protocol (20 methods) |
| `neve-fmt` | Code formatter (37/37 idempotency) |
| `neve-cli` | Command-line interface |
| `neve-diagnostic` | Error reporting (ariadne) |
| `neve-common` | Shared utilities + kill_process |
| `neve-store` | Content-addressed store (BLAKE3) |
| `neve-fetch` | Source fetching (URL, Git, local) |
| `neve-builder` | Sandbox build system |
| `neve-config` | System configuration (generation-based) |
| `neve-derive` | Derivation model + hashing |

## Current Status (v4.0.2)

| Metric | Value |
|--------|-------|
| Phase 6 (Syntax v4.0) | ✅ Complete |
| Phase 5 (Ecosystem) | ✅ Complete |
| Phase 4 (Shell) | ✅ Complete |
| E2E Tests | 541 (all pass) |
| Formatter idempotency | 37/37 |
| Stream<T> APIs | 14 (Phase A-C) |
| LSP Methods | 20 |
| EffectEval Rules | 34 (v4.3) |
| Lean Modules | 21 |
| Error Codes | 55 |
| Keywords | 12 (v4.0) |
| CI | All green |
| crates.io | Published (`cargo install neve`)|

## Decisions (2026-06-03)

- G2: Method dispatch is type-based; retain callable fallback
- Match: `match` must be exhaustive; non-exhaustive → `if-else`
- Lean: Continue full formal verification
- AST: compat path removed in v4.0 — HIR evaluator is canonical
- Release: SemVer-hybrid (see `docs/reference/stability.md`)
