# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Neve is a pure functional language for system configuration, built in Rust. 17 crates in a Cargo workspace. The CLI binary is `neve` (at `neve-cli/`). Published on crates.io. Install: `cargo install neve`.

## Build & Test

```bash
cargo build -p neve                     # build CLI
cargo check --workspace                 # fast validation
cargo test --workspace                  # all tests
cargo test --test end_to_end -- --nocapture  # 541 E2E tests
cargo test --test parser                # 220+ parser tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
.claude/skills/run-neve/driver.sh       # smoke-test all CLI paths
```

## Architecture

The canonical pipeline is `Lexer → Parser → HIR → Typeck → Eval`. All tooling (CLI, LSP, REPL) goes through `neve-frontend`.

```
crates/neve-lexer/     logos tokenizer
crates/neve-parser/    recursive descent LL(1)
crates/neve-syntax/    AST definitions
crates/neve-hir/       name resolution + lowering
crates/neve-typeck/    Hindley-Milner + traits
crates/neve-frontend/  pipeline facade (shared by CLI/LSP/REPL)
crates/neve-eval/      tree-walking interpreter (HIR canonical, AST deprecated)
crates/neve-std/       standard library (I/O, Stream<T>, collections)
crates/neve-lsp/       Language Server (20 methods)
crates/neve-fmt/       formatter
```

## Syntax v4.0

`let`/`fn`/`;` optional at top level. `use` not `import`. `|x|` not `fn(x)`. `{ }` records (no `#` prefix). `&` record merge and line comments. `type Foo = | A | B` for enums. 12 canonical keywords (+6 legacy aliases accepted by lexer).

v4.0 simplifications: `if cond -> a else b` (was `then`), `use p = alias` (was `as`), `~expr` (was `lazy`), `effect` auto-inferred, `pub` removed (all public).

## Key Rules

- **Code is truth.** Skills in `.claude/skills/` must match actual code. Run `.claude/hooks/verify-skills.sh` after API changes.
- **Canonical pipeline only.** Do not introduce new AST fallback paths. AST compat path (`neve_eval::compat`) has been removed in v4.0.
- **Every effectful builtin** needs: typeck entry + frontend wire-up + HIR eval + REPL `:type` + LSP hover + E2E parity. Update `is_effectful_builtin()` in `neve-std`.
- **Match must be exhaustive.** Use `if-else` for non-exhaustive conditions.
- **Conventional Commits:** `feat(scope):`, `fix(scope):`, `docs:`, `refactor:`, `release:`.

## Agent Infrastructure

`.claude/` contains the full development toolchain:
- `rules.md` — code-is-truth policy, verification requirements
- `skills/` — per-crate architecture references (parser, typeck, hir, eval, std, lsp, test, effect, lean)
- `skills/run-neve/` — smoke-test driver
- `hooks/` — pre-commit, verify-skills, check-regressions
- `forward-plan.md` — current status and Phase A→D roadmap
- `audit-report.md` — design audit findings (100% resolved, 62/62 ✅)

## Current Status

v4.0.2. 541 E2E tests (all pass). 20 LSP methods. 14 Stream<T> APIs. 53 error codes. 12 canonical keywords. Audit grade B+ → A- (62/62 fixed). All Phases complete. All CI green. Published on crates.io (`cargo install neve`).
