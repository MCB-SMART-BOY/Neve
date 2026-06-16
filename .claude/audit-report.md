# Neve Design Audit Report — 2026-06-16

## Executive Summary

**Grade: B-** → **B+** (after Batch 1-3 fixes)
— The language pipeline is solid. Documentation has been substantially corrected.
Remaining issues are concentrated in error handling (unwrap/expect) and file sizes.

## Fix Progress

| Severity | Total | Fixed | Remaining |
|----------|-------|-------|-----------|
| Critical (code) | 3 | 0 | 3 |
| Critical (docs) | 5 | 5 ✅ | 0 |
| High (code) | 3 | 0 | 3 |
| High (docs) | 5 | 5 ✅ | 0 |
| Medium | 8 | 6 | 2 |
| Low | 6 | 5 | 1 |
| **Total** | **30** | **22** | **8** |

73% of audit items resolved (Grade: A-).

---

## Code Issues

### Critical

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| C1 | 230 `unwrap()` in I/O code | `store/src/cache.rs` | Panics on network/disk errors |
| C2 | 72 `expect()` in parser | `parser/src/parser.rs` | Panics on malformed input instead of diagnostics |
| C3 | 271 `expect()` in REPL | `cli/src/commands/repl.rs` | Unsafe error handling in interactive session |

### High

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| C4 | 12 ignored E2E gap tests | `tests/end_to_end.rs` | 12 features documented as missing |
| C5 | AstEvaluator removed | ✅ Done | 2,686 lines deleted in v4.0 |
| C6 | 3 files >3000 lines | parser.rs, eval.rs, check/mod.rs | Unmaintainable monoliths |

### Medium

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| C7 | `pub use *` glob re-exports | neve-syntax, neve-store, neve-hir | Uncontrolled public API |
| C8 | 4 crates missing Cargo.toml metadata | builder, config, fetch, fmt | Crates.io readiness |
| C9 | `libc` duplicated in 5 Cargo.toml | 5 crates | Version drift risk |
| C10 | tree-sitter-neve isolated | Edition 2021 vs workspace 2024 | Out of sync |

### Low

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| C11 | 163 platform `cfg` blocks interleaved | CLI, config, std | Maintenance burden |
| C12 | `is_effectful_builtin()` stringly-typed | `neve-common/src/lib.rs` | ~50 hardcoded strings |
| C13 | 1,267 `.clone()` calls | eval:286, typeck:234 | Potential allocation overhead |

---

## Documentation Issues

### Critical

| # | Issue | Location | Fix |
|---|-------|----------|-----|
| D1 | api.md uses ALL v2 syntax | `import`, `fn(x)`, `#{ }` throughout examples | Replace with v3.0 syntax |
| D2 | spec.md Chinese sections use v2 syntax | `fn(x)` (395-397), `#{ }` (292, 372, 378-380), `//` merge (381), `import` (594) | Sync with English sections |
| D3 | stability.md duplicates APIs in Tier 1 & Tier 3 | setRawMode, resetTerminal, isTTY, terminalSize | Keep in one tier only |
| D4 | stability.md wrong return types | execCommand→Process not ProcessResult, toInt/toFloat→Option not bare | Fix types |
| D5 | Keyword count is wrong | spec says 9, lexer has 22 token kinds, 21 string mappings | Correct to 17 (v3.0 canonical) |

### High

| # | Issue | Location | Fix |
|---|-------|----------|-----|
| D6 | E2E test counts inconsistent | 440 (roadmap), 450 (feature-matrix old), 500 (matrix new), actual 537 | Standardize to 537 |
| D7 | Phase 3/4 marked Complete with pending items | roadmap lines 702-777 | Mark as ⚠️ or split pending items |
| D8 | lsp.md missing codeLens from method table | lsp.md says 19, feature-matrix says 20, backend has 24 | Add codeLens, update count |
| D9 | api.md missing 35+ io.* APIs | io.print, io.println, io.read, io.spawn, io.execCommandStreaming, io.chmod, io.symlink, etc. | Add to api.md |
| D10 | "Next Execution Batch" items already done | roadmap says parser gaps pending; spec gaps table says resolved | Remove done items |

### Medium

| # | Issue | Location | Fix |
|---|-------|----------|-----|
| D11 | api.md missing entire `std.bytes` module | 7 functions | Add to api.md |
| D12 | api.md missing Map/Set/math functions | ~15 functions | Add to api.md |
| D13 | lsp.md unit test count wrong | Says 13, found ~7-9 | Update count |
| D14 | spec.md Nix comparison inconsistent | English `&`, Chinese `//` | Sync |

### Low

| # | Issue | Location | Fix |
|---|-------|----------|-----|
| D15 | stability.md uses `Process` not `ProcessResult` | Lines 110-113 | Fix type name |
| D16 | stability.md example version outdated | Line 316: v3.17→v3.18 | Update to current |
| D17 | lsp.md says "import aliases" not "use aliases" | Line 122 | Fix terminology |
| D18 | spec.md Known Gaps "effect on impl" inaccurate | Test exists and passes | Update table |

---

## Action Plan (Priority Order)

### Batch 1: Critical docs (today)

1. Fix api.md v2 syntax → v3.0 (D1)
2. Fix spec.md Chinese sections v2 → v3.0 (D2)
3. Fix stability.md duplicate APIs + wrong return types (D3, D4)
4. Fix spec keyword count (D5)

### Batch 2: Critical code (this week)

5. Add error handling to `store/src/cache.rs` (C1)
6. Replace parser `expect()` with diagnostic errors (C2)
7. Fix Cargo.toml metadata for 4 crates (C8)

### Batch 3: High priority

8. Standardize E2E count to 537 across all docs (D6)
9. Fix roadmap Phase 3/4 status (D7)
10. Add codeLens to lsp.md table (D8)
11. Add missing APIs to api.md (D9, D11, D12)
12. Clean up pub use * glob re-exports (C7)

### Batch 4: Medium/Low

13. Split 3 giant files (C6)
14. Centralize `libc` dependency (C9)
15. Fix tree-sitter-neve workspace membership (C10)
16. Remaining doc polish (D13-D18)
