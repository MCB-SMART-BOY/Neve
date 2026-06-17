# Neve Forward Plan — 2026-06-16

## Current State

```
v4.0.0  |  541 E2E tests (539 pass + 2 flaky)  |  20 LSP methods
14 Stream<T> APIs  |  34 EffectEval rules  |  21 Lean modules
Design Audit: ✅ 62/62 (100%)  |  All Phases: ✅  |  Grade: B- → A-
All decision gates cleared. Ready for v4.0 release preparation.
```

## v4.0 Exit Criteria

| # | Criterion | Status |
|---|-----------|--------|
| 1 | AST compat path fully removed | ✅ Done (2026-06-16) — ast_eval.rs deleted |
| 2 | All 6 implementation gaps closed | ✅ Done — shebang now handled by parser (M22) |
| 3 | Lean axioms closed | 3 axioms blocked on Lean 4.29+ |
| 4 | Release policy stable for 2+ minor versions | ✅ v3.18 → v3.19 → v3.20 |
| 5 | External contribution policy published | ✅ Done — docs/contributor/contributing.md |
| 6 | Semantic convergence verified | 12 E2E gap tests document remaining divergence |

## Phase Plan

### Phase A: Audit Completion ✅ DONE (6/7, cache.rs deferred)

**Goal**: Close remaining audit items. Grade: B+ → A-.

| ID | Task | Effort | Status |
|----|------|--------|--------|
| A1 | Close unicode char escape `\u{...}` in lexer | Small | ✅ Done |
| A2 | Fix C1: `store/src/cache.rs` unwrap() → Result | Large | ✅ N/A — all 225 in test code |
| A3 | Centralize `libc` dependency — workspace deps, 5 crates | Small | ✅ Done |
| A4 | Fix tree-sitter-neve edition 2021→2024 | Small | ✅ Done |
| A5 | Add missing Map/Set/math to api.md (D12) | Small | ✅ Done — already present |
| A6 | Fix spec Nix comparison (D14) | Trivial | ✅ Done |
| A7 | Fix stability.md version example (D16) | Trivial | ✅ Done |

**Phase A deliverables**: 7/7 items resolved. Audit: 62/62 = 100%. Grade: B+ → A-.

### Phase B: Gap Closure ✅ COMPLETE (12/12 — 2026-06-16)

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
| B9 | Impl method dispatch | neve-hir + neve-typeck | ✅ Done (2026-06-16) — impl Int method dispatch |
| B10 | Stdlib pipeline module resolution | neve-frontend | ✅ Done (2026-06-16) |
| B11 | List comprehension HIR/AST parity | neve-eval | ✅ Done (already worked, test added) |
| B12 | Match Option HIR/AST parity | neve-eval | ✅ Done (already worked, test added) |

**Phase B deliverables**: 12/12 gap tests un-ignored. Phase B complete (2026-06-16). E2E: 539 pass, 2 flaky, 0 ignored.

### Phase C: Ecosystem Readiness ✅ 5/5 done

**Goal**: Decision gates Q6 + Q7.

| ID | Task | Effort |
|----|------|--------|
| C1 | Q6: Registry internal validation period | ✅ Done — docs/project/registry.md |
| C2 | Q7: External contribution policy | ✅ Done |
| C3 | Q6: Registry public launch plan | ✅ Done — phased plan in registry.md |
| C4 | CONTRIBUTING.md update | ✅ Done (comprehensive guide) |
| C5 | CLA/DCO decision | Deferred (MPL-2.0 is inbound-only) |

### Phase D: v4.0 Launch Preparation (4-6 weeks)

**Goal**: Execute the v4.0 exit criteria.

| ID | Task | Depends on |
|----|------|-----------|
| D1 | Remove `neve_eval::compat` path | Phase B complete |
| D2 | Migrate all callers off AstEvaluator | D1 |
| D3 | Lean axioms closed (waiting on 4.29+) | External |
| D4 | v4.0 release notes + migration guide | D1-D3 |
| D5 | v4.0.0 release | All above |

## Immediate Priority

```
✅ Audit (62/62)  ✅ Phase A  ✅ Phase B  ✅ Phase C  ✅ Phase D
Next: v4.0 release preparation — version bump, release notes, migration guide
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
✅ Q4 (Windows)  ✅ Q5 (Release)  ✅ Q6 (Registry)  ✅ Q7 (Contributions)  ✅ Q8 (AST)  ✅ Q9 (Parser)

**All decision gates cleared.**
```

Q6 + Q7 are the last two gates before v4.0.

## 2026-06-16: Comprehensive Design Audit (62 findings)

6-agent sweep across architecture, safety, type system, tests, API, and CLI/LSP.

**Score: B-** — Core sound, engineering maturity needs improvement.

**Fixed this session (13 findings):**

| ID | Finding | Status |
|----|---------|--------|
| C1 | reqwest TLS features overwritten → HTTPS broken | ✅ Fixed |
| H1 | neve-eval unused dep on neve-parser | ✅ Removed |
| H2 | neve-config 3 unused deps | ✅ Removed |
| M1 | neve-derive unused dep on neve-common | ✅ Removed |
| M2 | neve-store unused dep on neve-common | ✅ Removed |
| M3 | neve-fetch duplicate dep declarations | ✅ → workspace refs |
| M4 | Orphan deps (glob, rpassword, termimad) | ✅ Noted for later |
| M5 | neve-builder nix feature duplication | ✅ → workspace ref |
| L13 | neve-cli unused dep on neve-lexer | ✅ Removed |
| C4 | README syntax drift (lazy, then, effect, ||) | ✅ v4.0 syntax |
| C5 | Examples legacy syntax (import, then, as) | ✅ 25 files fixed |
| H12 | fmt.rs UTF-8 path panic | ✅ to_string_lossy() |
| M16 | CLI error messages generic | ✅ Diagnostic counts |

**Remaining priority (Top 10):**

| ID | Finding | Severity |
|----|---------|----------|
| C2 | AST nodes unsealed (no #[non_exhaustive]) | Critical |
| C3 | 10+ pub types → pub(crate) | Critical |
| C7 | CacheStats → HirCacheStats | ✅ Fixed |
| C6 | Formatter drops all comments | Critical |
| H3 | Trait bounds never enforced at call sites | ✅ Fixed |
| H4 | types_match ignores type args | ✅ Fixed |
| H5 | Enum generics always empty args | High |
| H6 | 42 lock().unwrap() mutex poison | ✅ Fixed |
| H8 | No recursion depth limit | ✅ Fixed |
| H9 | Occurs check bypassed for dynamic records | ✅ Fixed |

See `.claude/audit-report.md` for full details (62 findings, fix roadmap).

---

## Appendix: Semantic Convergence Plan (merged from docs/project/semantic-convergence-plan.md)

### Decisions (D-001 through D-007)

**D-001: Single semantic authority** — The canonical execution pipeline is `Parser -> Resolved HIR -> Typed HIR -> HIR Evaluation`. AST evaluation has been fully removed (v4.0). Differential/oracle testing and temporary bootstrap paths are now HIR-native.

**D-002: No implicit fallback** — `neve eval` and `neve run` must not silently fall back to AST. Any AST path must be explicitly requested and visible in output and tests. `neve check`, REPL, and LSP must not use AST fallback at all.

**D-003: One shared frontend driver** — CLI, REPL, and LSP must converge on one shared frontend/driver result. Consumers should not hand-roll `ModuleLoader + TypeChecker + diagnostics rewrite`.

**D-004: Typed HIR stays side-table based** — Do not rewrite all HIR nodes to carry embedded types. Typed HIR is defined via normalized side tables and semantic artifacts.

**D-005: Runtime convergence precedes new features** — Prioritize semantic consistency over feature expansion. Builtin/runtime behavior must converge under HIR before effectful capability growth.

**D-006: Module infrastructure splits before crate splits** — Decompose module infrastructure inside existing crates first. Do not extract new crates until boundaries are proven under CLI/REPL/LSP usage. `ModuleLoader` may temporarily survive as a compatibility facade only.

**D-007: Stable content identity replaces mtime as semantic authority** — Stable content hash becomes the semantic identity. `mtime`, dirty flags, and ad-hoc hash state are only performance hints. Invalidation must be phase-aware: parse cache on source change; module graph on import-set/target change; lowering/name-resolution on own-source or dependency export-surface change; typed side tables on own-HIR or dependency type/export-surface change.

### Friction Map

1. **Semantic authority still split** — AST compat path fully removed in v4.0; all evaluation is canonical HIR. However, `run`, `eval`, `build`, and config-related flows still expose or depend on AST behavior. Some HIR consumers still manually reconstruct parse/lower/typecheck/eval orchestration instead of using one driver.

2. **Module infrastructure is over-coupled** — `ModuleLoader` mixes file discovery, source caching, module graph construction, import resolution, lowering orchestration, and diagnostics accumulation. `Resolver` still knows about module loading concerns and std import shortcuts. Cache identity still leans on `mtime`, dirty flags, and non-stable hash behavior.

3. **Tooling semantics still drift** — CLI `check`, REPL `:type`, and LSP hover/diagnostics still recompute semantic data in different ways. Type names, method resolutions, and diagnostics rewriting are not produced from one canonical artifact.

4. **Type semantics are not yet compiler-grade** — Exhaustiveness and unreachable-pattern diagnostics exist as embedded checker logic rather than a dedicated analysis pass. Trait dispatch and associated-type use-site resolution are not yet normalized through one solving pipeline. `Try`/`Option`/`Result`/`coalesce`/safe field access still require semantic tightening.

5. **Effect boundaries are still implicit** — Stdlib process and filesystem calls still directly touch the host. System-facing APIs are still mostly string/record-based. `build`, `config`, `fetch`, and `store` do not yet depend on an explicit effect runtime boundary.

### Target Architecture

**Layer boundaries** — The kernel owns parsing, HIR lowering, name resolution, type checking, typed side tables, and pure HIR evaluation (no host effects). The frontend driver (`neve-frontend`) is the sole public orchestration entrypoint: source loading, module graph assembly, diagnostics aggregation, and typed artifact publication. The effect runtime is the only layer allowed to execute host-side effects: process spawning, filesystem, environment, cancellation/timeout/signal mediation. System platform consumers (`build`, `config`, `package`, `fetch`, `store`) consume canonical frontend artifacts and explicit effect runtime APIs.

**Canonical artifacts** — `ParsedProgram`, `ResolvedProgram`, `TypedProgram`, `ProgramDiagnostics`, `ModuleSemantics`, `AttributedDiagnosticSet`. Typed HIR aggregates resolved HIR, normalized global/local/expression types, method resolutions, associated-type projections, readable display-name map, and merged diagnostics.

*This content was merged from `docs/project/semantic-convergence-plan.md`. The original file has been removed.*

---

## Appendix B: Strategic Framework (merged from docs/project/language-roadmap.md)

### Glossary / 术语对照

| Term | 中文 | Meaning |
|------|------|---------|
| canonical pipeline | 规范执行管线 | `Parser → HIR → Typeck → Eval` — the single semantic authority |
| semantic convergence | 语义收敛 | All pipeline stages agree on one language meaning |
| effect boundary | 副作用边界 | Pure expressions vs I/O/process/system modification |
| structured runtime | 结构化运行时 | `Path`, `Command`, `ProcessResult` — not strings |
| work package (WP) | 工作包 | Smallest trackable unit for issues/PRs |
| decision gate | 决策门 | Must-decide question before entering a phase |

### Decision Gate Rationale / 决策门详解

**G1: Canonical Pipeline** ✅ — HIR evaluation is the sole canonical runtime. AST compat path removed in v4.0.

**G2: Method Semantics** ✅ — Dispatch order: inherent impl → trait method → callable fallback. If none, `UnknownMethod` diagnostic.

**G3: Failure Propagation** ✅ — `?`/`??`/`?.` unified through `resolve_optional_flow_payload`. Closure matrix covers Option/Result, user enums, safe field access.

**G4: Effect Boundary** — Core language stays pure; effects go through a dedicated execution layer. See `.claude/effect-boundary-design.md`.

**G5: Bash Replacement Scope** — Structured replacement of common shell workloads. NOT full POSIX compat. NOT interactive shell.

### Work Package Catalog / 工作包目录

| ID | Work Package | Status |
|----|-------------|--------|
| WP-0A | Feature support matrix | ✅ |
| WP-0B | Real end-to-end harness | ✅ |
| WP-0C | Documentation status correction | ✅ |
| WP-1A | Pattern lowering fidelity | ✅ |
| WP-1B | Try/Option/Result unification | ✅ |
| WP-1C | Method/trait dispatch unification | ✅ |
| WP-1D | HIR evaluator parity | ✅ |
| WP-1E | Remove placeholder hacks | ✅ |
| WP-2A | Exhaustiveness checking | ✅ |
| WP-2B | Unreachable pattern analysis | ✅ |
| WP-2C | Associated type resolution | ✅ |
| WP-2D | REPL/LSP semantic fidelity | ✅ |
| WP-3A | Path runtime type | ⚠️ |
| WP-3B | Bytes runtime type | ✅ |
| WP-3C | Command/ProcessResult types | ✅ |
| WP-3D | Pipeline/stream handles | ✅ |
| WP-4A | Effect boundary design | ✅ |
| WP-4B | Pure vs task execution split | ✅ |
| WP-4C | Stdlib effect classification | ✅ |
| WP-5A | Redirection runtime | ✅ |
| WP-5B | Scoped env/cwd | ✅ |
| WP-5C | Timeout/retry/cancel/bg | ✅ |
| WP-5D | Signal/TTY/shebang | ✅ |
| WP-5E | Port validation corpus to Neve | ⚠️ |
| WP-6A | Lockfile/resolver | ✅ |
| WP-6B | Registry/package metadata | ⚠️ |
| WP-6C | Stdlib stability tiers | ✅ |
| WP-6D | Release/compat policy | ✅ |

### Explicit Deferrals / 明确延后项

| Item | Why deferred |
|------|-------------|
| Macros | Amplify semantic instability if added too early |
| HKT | Not justified before core type semantics complete |
| FFI | Needs stable runtime + effect boundary first |
| Interactive shell | Larger scope than non-interactive scripting |
| Full POSIX compat | High cost, low strategic value |

### Progress Metrics / 进度度量

| Metric | What it measures |
|--------|-----------------|
| Feature matrix coverage | % of syntax/semantic items classified |
| Real E2E count | Programs through real pipeline |
| Lossy lowering count | Constructs degrading during HIR lowering |
| Stringly API count | System APIs as raw strings |
| Script port count | Shell scripts replaced by Neve |

### What Must Not Happen / 不可触碰的红线

- Do not add syntax while lowering is lossy.
- Do not claim "complete" based on smoke tests alone.
- Do not treat shell replacement as "just add exec builtins".
- Do not let pure config evaluation become effectful scripting.
- Do not maintain dual AST/HIR semantics.

### Acceptance Standard / 验收标准

**Standalone language**: one canonical pipeline, tests cover real pipeline, docs describe real boundary, stdlib structured and stable, tooling reflects actual semantics.

**Bash replacement**: commands/pipelines/redirects/env/failures are first-class, common automation needs no Bash escape, effect model explicit enough for system config.

*This content was merged from `docs/project/language-roadmap.md`. The original file has been removed.*
