# neve-effect: Effect System

## Architecture

```
Source: fn f() = io.readFile("config.neve")
       │
       ▼
┌────────────────────────────────────┐
│  1. Parser                         │
│  Records call to "io.readFile"    │
│  Marks as effectful builtin       │
└──────────────┬─────────────────────┘
               ▼
┌────────────────────────────────────┐
│  2. HIR Lowering                   │
│  Resolves io.readFile → Effect::IO │
│  Wraps call in PerformEffect node │
└──────────────┬─────────────────────┘
               ▼
┌────────────────────────────────────┐
│  3. Type Checker                   │
│  Infers: f: () → String / IO      │
│  Propagates effect to callers     │
│  Checks pure/effect boundaries    │
└──────────────┬─────────────────────┘
               ▼
┌────────────────────────────────────┐
│  4. Evaluator                      │
│  handle_effect(EffectId::IO, arg)  │
│  Dispatches to builtin impl        │
└────────────────────────────────────┘
```

## Effect Taxonomy

```rust
pub enum Effect {
    Pure,           // No side effects (default)
    IO,             // File I/O, directory ops, environment
    Process,        // Command/Pipeline execution
    Async,          // Task/Spawn/Await
    Network,        // Fetch URL/Git
    FileSystem,     // File metadata, symlinks, chmod
    Custom(Name),   // User-defined effects (future)
}

pub struct EffectSet {
    effects: BTreeSet<Effect>,
}
```

## Effect Propagation

```
pure function
    │
    ├── calls pure function    → EffectSet::Pure ✅
    ├── calls io.readFile      → EffectSet::IO     ← propagated to caller
    └── calls io.spawn         → EffectSet::Async  ← propagated to caller

Caller must also be annotated (effect auto-inferred in v3.0).
```

## EffectEval v4.3 (Lean Formalization)

34 rules across 7 categories:

| Category | Rules | Coverage |
|----------|-------|----------|
| Pure | 1 | `pure` — lift BigStep, σ unchanged |
| Blocking Process | 2 | `execCommand`, `execPipeline` |
| Deferred Task | 6 | `spawn`, `awaitTask`, `awaitWithTimeout`, `awaitTasks`, `cancel`, `awaitAny` |
| Streaming | 6 | `execCommandStreaming`, `execPipelineStreaming`, streaming timeouts (×2 success + ×2 expired) |
| File I/O | 4 | `readFile`, `writeFile`, `readFileBytes`, `writeFileBytes` |
| Stream<T> | 5 | `streamCollect`, `streamPipe`, `streamForEach`, `streamFold`, `streamWithTimeout` |
| Retry/Ensure | 4 | `retrySuccess`, `retryFailure`, `ensureSuccess`, `ensureTimeout` |
| **Total** | **34** | |

## Size Limits (Mandatory in EffectEval Rules)

```
MAX_STDIN_BYTES   = 10 MB   ← enforced in all Command/Pipeline exec rules
MAX_OUTPUT_BYTES  = 50 MB   ← enforced in all blocking execution rules
MAX_STREAM_LINES  = 100k    ← enforced in streaming rule premises
```

These are **premises** in the Lean formalization — every valid derivation tree is a proof that limits were respected.

## Security Properties (Machine-Checked)

| Finding | Property | Lean Theorem |
|---------|----------|-------------|
| H-1 | stdin ≤ 10MB | `Verify/Limits.lean` |
| H-2 | output ≤ 50MB | `Verify/Limits.lean` |
| M-1 | Path traversal blocked | `Verify/Path.lean` |
| M-2 | Unified kill mechanism | `neve-common::kill_process` |
| M-4 | Env injection stripped | `Verify/Environ.lean` |

## Effect Checking in CLI

```bash
neve check file.neve           # Pure mode (default) — rejects effectful calls
neve check --allow-effects     # Permissive mode — allows effects
```

## Integration Points

| Stage | File | What It Does |
|-------|------|-------------|
| Parser | `neve-parser/src/parser.rs` | Records effect keyword (backward compat) |
| HIR | `neve-hir/src/lower.rs` | Wraps effectful calls in `HIRExpr::PerformEffect` |
| Typeck | `neve-typeck/src/effects.rs` | Infers and checks effect propagation |
| Eval | `neve-eval/src/eval.rs` | `handle_effect()` dispatches to builtins |
| Stdlib | `neve-std/src/lib.rs` | `is_effectful_builtin()` — canonical registry |
| Lean | `formal/Neve/Spec/Effects.lean` | EffectEval v4.3 (34 rules) |

## Key Files

| File | What |
|------|------|
| `typeck/src/effects.rs` | Effect inference in type checker |
| `eval/src/eval.rs` → `handle_effect()` | Effect dispatch in HIR evaluator |
| `eval/src/ast_eval.rs` → effect handling | Effect dispatch in AST compat (deprecated) |
| `std/src/lib.rs` → `is_effectful_builtin()` | Canonical effect classification |
| `std/src/io/mod.rs` | I/O effect implementations |
| `cli/src/commands/check.rs` | `neve check` purity enforcement |
| `formal/Neve/Spec/Effects.lean` | EffectEval v4.3 formalization |
| `docs/project/effect-boundary.md` | Effect boundary design document |
