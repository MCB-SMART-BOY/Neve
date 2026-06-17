# neve-dev: Project Architecture & Development

## Crate Dependency Graph

```
neve-cli ──────┬── neve-frontend ───┬── neve-parser ─── neve-lexer
               │                    │                   neve-syntax
               │                    ├── neve-hir
               │                    ├── neve-typeck
               │                    └── neve-eval ───── neve-std
               │
               ├── neve-lsp ──────── neve-frontend
               ├── neve-fmt ──────── neve-parser + neve-lexer
               ├── neve-config ───── neve-eval
               ├── neve-builder ──── neve-store + neve-fetch + neve-derive
               └── neve-diagnostic ─ (all crates)
```

**Dependency law**: `neve-frontend` is the **single entry point** for the language pipeline. No crate except `neve-cli`, `neve-lsp`, and `neve-config` should directly construct the parser + HIR + typeck chain.

## Data Flow Through the Pipeline

```
User Input (.neve file or REPL line)
       │
       ▼
┌──────────────────────────────────────────────┐
│  neve-frontend (Driver + Session)             │
│  ┌─────────────────────────────────────────┐ │
│  │ 1. Lex  → [Token]                       │ │
│  │ 2. Parse → AST Module                   │ │
│  │ 3. Lower → Resolved HIR                 │ │
│  │ 4. Check → Typed HIR                    │ │
│  │ 5. Eval  → Value                        │ │
│  └─────────────────────────────────────────┘ │
│  Side tables: diagnostics, type map, def map  │
└──────────────────────────────────────────────┘
       │
       ├──→ CLI output (run/eval/check)
       ├──→ LSP responses (hover/completion/diag)
       └──→ Config evaluation (flake.nix-like)
```

## Build Commands

```bash
cargo build -p neve         # CLI binary (CI target)
cargo check --workspace     # Fast validation (no codegen)
cargo test --workspace      # Unit + integration
cargo test --test end_to_end -- --nocapture  # E2E
cargo fmt --all             # Format (enforced in CI)
cargo clippy --workspace --all-targets -- -D warnings  # Lint
./scripts/test.sh --clippy  # Pre-commit pipeline
```

## Commit Conventions

```
feat(repl): add history persistence    # New feature
fix(ci): correct script paths          # Bug fix
docs: update changelog                 # Documentation
refactor(typeck): simplify unification # Refactor
style(fmt): trailing commas            # Style only
release: bump version to 3.20.0        # Release
```

## Feature Addition Checklist

Every new effectful builtin:
1. `neve-std`: Register in `is_effectful_builtin()`
2. `neve-typeck`: Add type signature
3. `neve-frontend`: Wire into pipeline
4. `neve-eval`: Implement HIR evaluation
5. REPL: `:type` support
6. LSP: hover + completion
7. `tests/end_to_end.rs`: E2E parity test
8. `docs/reference/api.md`: Document
9. `feature-matrix.md`: Update status

## Architecture Decisions

| Decision | When | Why |
|----------|------|-----|
| HIR as canonical pipeline | v1.2 | Single semantic truth, no AST/HIR divergence |
| `neve-frontend` as facade | v1.2 | Share analysis across CLI + LSP + REPL |
| AST compat path removed | v4.0 (2026-06) | HIR evaluator is the only path; ~3500 lines deleted |
| SemVer-hybrid release | v3.19 | Rapid evolution with clear deprecation lifecycle |
| `match` must be exhaustive | v3.18+ | Compiler-grade safety; if-else for non-exhaustive |

## Project Status

| Phase | Status | Key Deliverable |
|-------|--------|----------------|
| Syntax v4.0 | ✅ | 12 keywords, `if->`, `use=`, `~expr`, effect auto |
| Phase 5 (Ecosystem) | ✅ | Flake/lock/store, Registry v1, crates.io |
| Phase 4 (Shell) | ✅ | Stream<T> 14 APIs, Task, TTY |
| Phase 3 (Runtime) | ✅ | Path/Bytes/Command/ProcessResult |
| Phase 2 (Type) | ✅ | Exhaustive match, trait dispatch |
| Phase 1 (Convergence) | ✅ | Canonical HIR pipeline |
