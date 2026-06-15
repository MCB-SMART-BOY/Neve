# Neve Forward Plan — 2026-06-16

## Current State

```
v3.18.0+  |  541 E2E tests (538 pass + 2 fail + 1 gap)  |  20 LSP methods
14 Stream<T> APIs  |  34 EffectEval rules  |  19 Lean modules
Audit: 22/30 fixed (73%)  |  Keywords: 12 (v4.0)  |  Phase B: 11/12 gaps closed
```

## v4.0 Exit Criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | AST compat path fully removed | Not started — AstEvaluator still used in 5 files |
| 2 | All 6 implementation gaps closed | 2/6 remain (unicode char, shebang parser) |
| 3 | Lean axioms closed | 3 axioms blocked on Lean 4.29+ |
| 4 | Release policy stable for 2+ minor versions | ✅ v3.18 → v3.19 → v3.20 |
| 5 | External contribution policy published | Not started (Q7) |
| 6 | Semantic convergence verified | 12 E2E gap tests document remaining divergence |

## Phase Plan

### Phase A: Audit Completion ✅ DONE (6/7, cache.rs deferred)

**Goal**: Close remaining audit items. Grade: B+ → A-.

| ID | Task | Effort | Status |
|----|------|--------|--------|
| A1 | Close unicode char escape `\u{...}` in lexer | Small | ✅ Done |
| A2 | Fix C1: `store/src/cache.rs` unwrap() → Result | Large | ⬜ Deferred |
| A3 | Centralize `libc` dependency — workspace deps, 5 crates | Small | ✅ Done |
| A4 | Fix tree-sitter-neve edition 2021→2024 | Small | ✅ Done |
| A5 | Add missing Map/Set/math to api.md (D12) | Small | ⬜ Deferred |
| A6 | Fix spec Nix comparison (D14) | Trivial | ✅ Done |
| A7 | Fix stability.md version example (D16) | Trivial | ✅ Done |

**Phase A deliverables**: 5/7 items fixed. 1 spec gap closed (unicode char). Audit: 22/30 = 73%.

### Phase B: Gap Closure (11/12 gaps closed)

**Goal**: Close the 12 E2E gap tests. These represent real missing features.

The 12 gaps, ordered by impact:

| # | Gap | Crate to fix | Status |
|---|-----|-------------|--------|
| B1 | v3.0 enum pipe syntax (`\| Red \| Green \| Blue`) | neve-parser | ✅ Done (2026-06-16) |
| B2 | TupleIndex expression | neve-eval (HIR) | ✅ Done (already worked, test added) |
| B3 | Block-with-let lowering | neve-hir | ✅ Done (already worked, test added) |
| B4 | Nested blocks lowering | neve-hir | ✅ Done (already worked, test added) |
| B5 | Generic identity inference | neve-typeck | ✅ Done (2026-06-16) — generalize + instantiate fix |
| B6 | Option match pattern lowering | neve-hir | ✅ Done (already worked, test added) |
| B7 | Record match pattern lowering | neve-hir | ✅ Done (works with #{ } syntax; v3.0 { } pattern is parser gap) |
| B8 | `?.` safe access lowering | neve-hir | ✅ Done (already worked, test added) |
| B9 | Impl method dispatch | neve-hir + neve-typeck | ⬜ Remaining — self parameter type resolution needed |
| B10 | Stdlib pipeline module resolution | neve-frontend | ✅ Done (2026-06-16) |
| B11 | List comprehension HIR/AST parity | neve-eval | ✅ Done (already worked, test added) |
| B12 | Match Option HIR/AST parity | neve-eval | ✅ Done (already worked, test added) |

**Phase B deliverables**: 10/12 gap tests un-ignored. 2 remaining: B5 (polymorphism) + B9 (impl dispatch). E2E: 537 → 541 tests.

### Phase C: Ecosystem Readiness (2-4 weeks, parallel with B)

**Goal**: Decision gates Q6 + Q7.

| ID | Task | Effort |
|----|------|--------|
| C1 | Q6: Registry internal validation period | Documentation |
| C2 | Q7: External contribution policy | Documentation |
| C3 | Q6: Registry public launch plan | Documentation |
| C4 | CONTRIBUTING.md update | Documentation |
| C5 | CLA/DCO decision | Decision |

### Phase D: v4.0 Launch Preparation (4-6 weeks)

**Goal**: Execute the v4.0 exit criteria.

| ID | Task | Depends on |
|----|------|-----------|
| D1 | Remove `neve_eval::compat` path | Phase B complete |
| D2 | Migrate all callers off AstEvaluator | D1 |
| D3 | Lean axioms closed (waiting on 4.29+) | External |
| D4 | v4.0 release notes + migration guide | D1-D3 |
| D5 | v4.0.0 release | All above |

## Immediate Priority (this week)

```
1. B5: Fix multi-call-site polymorphism instantiation   ← typeck fix
2. B9: Fix inherent impl method dispatch                ← HIR+typeck fix
3. A2: Begin cache.rs unwrap() fix                      ← start the big one
```

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Lean 4.29+ delayed beyond v4.0 window | Medium | Low | v4.0 can ship with 3 documented axioms |
| AstEvaluator migration breaks CLI paths | High | High | Migrate one caller at a time, test each |
| cache.rs unwrap() fix introduces regressions | Medium | Medium | Add tests before refactoring |
| Parser expect() fix destabilizes error messages | Low | High | Defer to post-v4.0 |

## Decision Gates Remaining

```
✅ Q8 (AST deprecation)  ✅ Q9 (Parser gaps)  ✅ Q5 (Release policy)
✅ Q4 (Windows tiers)    ⬜ Q6 (Registry)      ⬜ Q7 (Contributions)
```

Q6 + Q7 are the last two gates before v4.0.
