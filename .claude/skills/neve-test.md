# neve-test: Testing Strategy & Architecture

## Test Pyramid

```
           ┌──────────┐
           │ E2E      │  450 tests — full pipeline smoke
           │ (smoke)  │  tests/end_to_end.rs
           ├──────────┤
           │ Inte-    │  ~600 tests — subsystem validation
           │ gration  │  tests/parser.rs, typeck.rs, eval.rs, ...
           ├──────────┤
           │  Unit    │  500+ tests — in-module #[test]
           │          │  */src/*.rs
           └──────────┘
```

## Test Architecture

### Unit Tests (in-crate)
Every crate has `#[cfg(test)] mod tests` blocks. Tests are co-located with the code they test.

```rust
// crates/neve-typeck/src/infer.rs
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_infer_lambda() { ... }
    #[test]
    fn test_occurs_check() { ... }
}
```

### Integration Tests (tests/*.rs)

| Test File | What It Validates | Count |
|-----------|-------------------|-------|
| `tests/parser.rs` | Lexer + parser for all syntax forms | 220+ |
| `tests/typeck.rs` | Type inference, traits, exhaustiveness | 287+ |
| `tests/end_to_end.rs` | Full pipeline: parse→typeck→eval | 450 |
| `tests/eval.rs` | AST compat evaluation path | ~50 |
| `tests/lsp_e2e.rs` | LSP protocol conformance | 8 |
| `tests/syntax_policy.rs` | Semantic policy assertions | 4 |
| `tests/std_root_imports.rs` | Stdlib import resolution | 3 |
| `tests/store.rs` | Content-addressed store | 55 |
| `tests/formatter.rs` | Formatter idempotency | 37 |

## E2E Test Architecture

```rust
// tests/end_to_end.rs — canonical pipeline smoke

fn run(source: &str) -> Result<Value, EvalError> {
    let ast = neve_frontend::parse_source(source, "<test>")?;
    let hir = neve_frontend::lower_module(&ast)?;
    let typed = neve_frontend::typecheck_module(&hir)?;
    neve_eval::Evaluator::new().eval_module(&typed)
}

fn run_ast(source: &str) -> Result<Value, EvalError> {
    // Deprecated path — still tested for parity
    #[allow(deprecated)]
    neve_eval::compat::AstEvaluator::new()
        .with_module_overrides(std_module_overrides())
        .eval_source(source)
}
```

## E2E Coverage (450 tests)

| Category | Tests | Examples |
|----------|-------|----------|
| Arithmetic | 60+ | Int/Float ops, precedence, overflow |
| Lists | 50+ | map, filter, fold, comprehension, concat |
| Records | 40+ | field access, update, merge, shorthand |
| Functions | 45+ | lambda, closure, recursion, currying |
| Pattern matching | 35+ | exhaustive, guards, or-patterns, binding |
| Optional flow | 25+ | `?`, `??`, `?.`, Option, Result |
| Modules | 20+ | use, visibility, relative paths |
| I/O | 30+ | readFile, writeFile, exec, redirect |
| Streams | 30+ | 14 Stream<T> APIs |
| Tasks | 20+ | spawn, poll, cancel, awaitAny |
| Signals | 8+ | SIGINT, SIGTERM, SIGUSR1 |
| TTY | 5+ | raw mode, readKey, terminalSize |
| Glob | 5+ | file pattern matching |
| Bytes | 10+ | binary data handling |
| Formatter | 37 | idempotency round-trips |
| Shebang | 3+ | script entrypoints |

## Differential Testing

```
Random expression generator
       │
       ├──→ HIR evaluator → Value A
       │
       └──→ AST evaluator → Value B
                │
                ▼
          assert_eq!(A, B)   ← 300+ random tests (ALL MATCH)
```

## Bug Hunter

```
┌────────────────────────────────────────────┐
│ Fuzzer — generates valid + invalid programs │
│                                            │
│ Tests:                                     │
│  1. No crashes on malformed input          │
│  2. Security boundaries (H-1, H-2, M-1,   │
│     M-2, M-4) never violated               │
│  3. Path sanitization holds                │
│  4. Buffer limits enforced                 │
└────────────────────────────────────────────┘
```

## Test Commands

```bash
cargo test --workspace                     # Everything
cargo test --test end_to_end               # E2E only
cargo test --test parser                   # Parser only
cargo test --test typeck                   # Type checker only
cargo test --test end_to_end -- --nocapture  # With output
cargo test -p neve-parser                   # Crate unit tests
./scripts/test.sh --clippy                  # Pre-commit
./scripts/test.sh --differential            # 300+ parity tests
./scripts/test.sh --bug-hunt                # Fuzzing
```

## Testing New Features

```
Implementation order:
  1. Unit test (red)  → in-module, tests the function directly
  2. Implement (green) → write the feature
  3. Integration test → in tests/<area>.rs, tests subsystem behavior
  4. E2E smoke test   → in tests/end_to_end.rs, validates full pipeline
  5. Update feature-matrix.md → reflect capability status
```

## Key Files

| File | What |
|------|------|
| `tests/end_to_end.rs` | 450 canonical pipeline tests |
| `tests/parser.rs` | 220+ parser integration tests + golden tests |
| `tests/typeck.rs` | 287+ type system tests |
| `tests/eval.rs` | AST compat evaluation tests |
| `tests/README.md` | Test policy — gap naming, divergence sentinels |
| `scripts/test.sh` | Pre-commit pipeline |
| `scripts/bug_hunt.py` | Fuzzing harness |
