# n3v3-test: Testing Strategy & Architecture

## Test Pyramid

```
           ┌──────────┐
           │ E2E      │ 556 E2E tests (all pass) — full pipeline smoke
           │ (smoke)  │  tests/end_to_end.rs
           ├──────────┤
           │ Inte-    │  Subsystem validation — see tests/*.rs
           │ gration  │
           ├──────────┤
           │  Unit    │  In-crate #[test] modules
           │          │  */src/*.rs
           └──────────┘
```

## Test Architecture

### Unit Tests (in-crate)
Every crate has `#[cfg(test)] mod tests` blocks. Tests are co-located with the code they test.

```rust
// crates/n3v3-typeck/src/infer.rs
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

| Test File | What It Validates | Verification |
|-----------|-------------------|--------------|
| `tests/parser.rs` | Lexer + parser for all syntax forms | `cargo test --test parser` (234 parser tests) |
| `tests/typeck.rs` | Type inference, traits, exhaustiveness | `cargo test --test typeck` |
| `tests/end_to_end.rs` | Full pipeline: parse→typeck→eval | `cargo test --test end_to_end` (556 E2E tests) |
| `tests/eval.rs` | HIR evaluator unit tests | `cargo test --test eval` |
| `tests/lsp_e2e.rs` | LSP protocol conformance | `cargo test --test lsp_e2e` |
| `tests/syntax_policy.rs` | Semantic policy assertions | `cargo test --test syntax_policy` |
| `tests/std_root_imports.rs` | Stdlib import resolution | `cargo test --test std_root_imports` |
| `tests/store.rs` | Content-addressed store | `cargo test --test store` |
| `tests/fmt.rs` | Formatter idempotency and syntax | `cargo test --test fmt` |

## E2E Test Architecture

```rust
// tests/end_to_end.rs — canonical pipeline smoke

fn run(source: &str) -> Result<Value, EvalError> {
    let analysis = n3v3_frontend::analyze_source(source);
    n3v3_eval::Evaluator::new().eval_module(&analysis.hir)
}

// Every eval path goes through the canonical HIR pipeline.
// The AST compat path was fully removed in v4.0 (Phase D).
```

## E2E Coverage (556 E2E tests)

| Category | Representative coverage |
|----------|-------------------------|
| Arithmetic | Int/Float ops, precedence, overflow |
| Lists | map, filter, fold, comprehension, concat |
| Records | field access, update, merge, shorthand |
| Functions | lambda, closure, recursion, currying |
| Pattern matching | exhaustive, guards, or-patterns, binding |
| Optional flow | `?`, `??`, `?.`, Option, Result |
| Modules | use, visibility, relative paths |
| I/O | readFile, writeFile, exec, redirect |
| Streams | 13 Stream<T> APIs |
| Tasks | spawn, poll, cancel, awaitAny |
| Signals | SIGINT, SIGTERM, SIGUSR1 |
| TTY | raw mode, readKey, terminalSize |
| Glob | file pattern matching |
| Bytes | binary data handling |
| Formatter | idempotency round-trips |
| Shebang | script entrypoints |

```bash
cargo test --workspace                     # Everything
cargo test --test end_to_end               # E2E only
cargo test --test parser                   # Parser only
cargo test --test typeck                   # Type checker only
cargo test --test end_to_end -- --nocapture  # With output
cargo test -p n3v3-parser                   # Crate unit tests
./scripts/test.sh --clippy                  # Pre-commit
./scripts/test.sh --hunt                    # Fuzzing
```

## Testing New Features

```
Implementation order:
  1. Unit test (red)  → in-module, tests the function directly
  2. Implement (green) → write the feature
  3. Integration test → in tests/<area>.rs, tests subsystem behavior
  4. E2E smoke test   → in tests/end_to_end.rs, validates full pipeline
  5. Update docs/project/feature-matrix.md → reflect capability status
```

## Key Files

| File | What |
|------|------|
| `tests/end_to_end.rs` | 556 E2E tests through the canonical pipeline |
| `tests/parser.rs` | 234 parser integration tests + golden tests |
| `tests/typeck.rs` | Type system and exhaustiveness tests |
| `tests/frontend.rs` | Effect propagation and frontend pipeline regressions |
| `tests/lsp.rs` | LSP semantic hover, symbol, and navigation regressions |
| `tests/eval.rs` | HIR evaluator tests |
| `tests/runtime_semantics.rs` | Lazy-parameter and evaluator-scope regressions |
| `scripts/test.sh` | Pre-commit pipeline |
| `scripts/bug-hunt.n3v3` | Fuzzing harness |
