# n3v3-effect: Effect System

## Architecture

```
Source: fn f() = io.readFile("config.n3v3")
       │
       ▼
┌────────────────────────────────────┐
│  1. Parser                         │
│  Records a call to a builtin name  │
└──────────────┬─────────────────────┘
               ▼
┌────────────────────────────────────┐
│  2. HIR Lowering                   │
│  Resolves it to ExprKind::Builtin  │
│  and records the canonical call    │
└──────────────┬─────────────────────┘
               ▼
┌────────────────────────────────────┐
│  3. Type Checker                   │
│  Marks effectful builtins and      │
│  propagates effects to callers     │
└──────────────┬─────────────────────┘
               ▼
┌────────────────────────────────────┐
│  4. Evaluator                      │
│  Resolves builtin_value(name) and  │
│  dispatches through the call path  │
└────────────────────────────────────┘
```

## Effect Taxonomy

The implementation records effectfulness on HIR function definitions and on the
type checker's effectful-definition set. Builtin classification is centralized
in `n3v3_common::is_effectful_builtin`; `n3v3_std` delegates to that registry.

```rust
pub struct FnDef {
    pub effectful: bool,
    // name, params, return_ty, body, ...
}

fn is_effectful_builtin(name: &str) -> bool {
    n3v3_common::is_effectful_builtin(name)
}
```

## Effect Propagation

```
pure function
    │
    ├── calls a pure function    → remains pure
    ├── calls io.readFile        → function becomes effectful
    └── calls io.spawn           → function becomes effectful

The type checker propagates effectful definitions through direct calls and
resolved method calls before `n3v3 check` enforces the effect boundary.
```

## EffectEval 34 Rules (Lean Formalization)

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
| M-2 | Unified kill mechanism | `n3v3-common::kill_process` |
| M-4 | Env injection stripped | `Verify/Environ.lean` |

## Effect Checking in CLI

```bash
n3v3 check file.n3v3           # Effect check enabled by default
n3v3 check --allow-effects file.n3v3 # Allow effectful calls
```

`n3v3 check` fails on `Severity::Error` diagnostics (including effect-check
failures). `Severity::Warning` diagnostics do not make the command fail; a
clean check prints `[OK] OK - No errors found`.

## Integration Points

| Stage | File | What It Does |
|-------|------|-------------|
| Parser | `crates/n3v3-parser/src/parser.rs` | Parses builtin calls (legacy `effect` syntax remains accepted) |
| HIR | `crates/n3v3-hir/src/resolve.rs` | Resolves builtin names to `ExprKind::Builtin` |
| Typeck | `crates/n3v3-typeck/src/check/mod.rs` | Infers and checks effect propagation |
| Eval | `crates/n3v3-eval/src/eval.rs` | Resolves and applies builtin values |
| Stdlib | `crates/n3v3-std/src/lib.rs` | Delegates effect classification to the shared registry |
| Lean | `formal/n3v3/Spec/Effects.lean` | EffectEval v4.3 (34 rules) |

## Key Files

| File | What |
|------|------|
| `crates/n3v3-typeck/src/check/mod.rs` | Effect inference in the type checker |
| `crates/n3v3-eval/src/eval.rs` | Builtin/effect dispatch in the HIR evaluator |
| `crates/n3v3-std/src/lib.rs` | Standard-library builtin facade |
| `crates/n3v3-std/src/io/mod.rs` | I/O effect implementations |
| `n3v3-cli/src/commands/check.rs` | `n3v3 check` effect enforcement |
| `formal/n3v3/Spec/Effects.lean` | EffectEval v4.3 formalization |
| `.claude/effect-boundary-design.md` | Effect boundary design document |
