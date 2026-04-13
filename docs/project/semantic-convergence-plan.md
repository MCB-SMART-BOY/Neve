# Neve Semantic Convergence Plan

This file is the living execution plan for semantic convergence around Neve's canonical pipeline:

`Parser -> Resolved HIR -> Typed HIR -> HIR Evaluation`

From this point onward, implementation work for semantic convergence should read this file first, update it when direction changes, and treat it as the default source of truth before continuing code changes.

## 1. Status

- State: active
- Scope: semantic convergence, runtime boundary clarity, tooling consistency
- Default bias:
  - close semantic loops before expanding language surface
  - prefer one canonical implementation over parallel "temporary" paths
  - keep migrations incremental and reviewable

## 2. North Star

Neve is considered on the mainline only when:

- one canonical semantic pipeline defines user-visible behavior
- AST evaluation is no longer a co-equal semantic authority
- CLI, REPL, and LSP consume one shared semantic artifact
- system effects are explicit and runtime-mediated
- public docs describe the real behavior rather than intended behavior
- full-pipeline tests validate the real path used by users

## 3. Non-Goals

- no big-bang rewrite
- no hidden fallback
- no new language surface syntax unless it is unavoidable and explicitly approved
- no HKT, macro system, FFI, full shell compatibility, or other expansion topics before the current semantic core closes

## 4. Current Friction Map

### 4.1 Semantic authority still split

- AST and HIR evaluators still coexist in user-visible paths.
- `run`, `eval`, `build`, and config-related flows still expose or depend on AST behavior.
- Some HIR consumers still manually reconstruct parse/lower/typecheck/eval orchestration instead of using one driver.

### 4.2 Module infrastructure is over-coupled

- `ModuleLoader` currently mixes:
  - file discovery
  - source caching
  - module graph construction
  - import resolution
  - lowering orchestration
  - diagnostics accumulation
- `Resolver` still knows about module loading concerns and std import shortcuts.
- Cache identity still leans on `mtime`, dirty flags, and non-stable hash behavior.

### 4.3 Tooling semantics still drift

- CLI `check`, REPL `:type`, and LSP hover/diagnostics still recompute semantic data in different ways.
- Type names, method resolutions, and diagnostics rewriting are not produced from one canonical artifact.

### 4.4 Type semantics are not yet compiler-grade

- exhaustiveness and unreachable-pattern diagnostics exist, but still as embedded checker logic rather than a dedicated analysis pass
- trait dispatch and associated-type use-site resolution are not yet normalized through one solving pipeline
- `Try` / `Option` / `Result` / `coalesce` / safe field access still require semantic tightening

### 4.5 Effect boundaries are still implicit

- stdlib process and filesystem calls still directly touch the host
- system-facing APIs are still mostly string/record-based
- `build`, `config`, `fetch`, and `store` do not yet depend on an explicit effect runtime boundary

## 5. Target Architecture

## 5.1 Layer boundaries

### Kernel

The kernel is the only semantic core and owns:

- parsing
- HIR lowering and name resolution
- type checking
- typed semantic side tables
- pure HIR evaluation

The kernel does not directly perform filesystem, environment, process, or network effects.

### Frontend Driver

`neve-frontend` becomes the only public orchestration entrypoint and owns:

- source loading/session management
- module graph assembly
- diagnostics aggregation and attribution
- resolved HIR program assembly
- typed semantic artifact publication

### Effect Runtime

The effect runtime is the only layer allowed to execute host-side effects. It owns:

- process spawning and monitoring
- filesystem interactions
- environment access and mutation
- cancellation / timeout / signal mediation
- typed runtime object execution

### System Platform

`build`, `config`, `package`, `fetch`, and `store` become platform consumers of:

- canonical frontend artifacts
- explicit effect runtime APIs

They do not define an alternative language semantics.

## 5.2 Canonical program artifacts

The main semantic artifacts should converge toward:

- `ParsedProgram`
- `ResolvedProgram`
- `TypedProgram`
- `ProgramDiagnostics`
- `ModuleSemantics`
- `AttributedDiagnosticSet`

`Typed HIR` remains side-table based in this migration. It means:

- resolved HIR
- normalized global types
- normalized local/expression types
- method resolutions
- associated-type projection resolutions
- readable display-name map
- merged diagnostics

## 6. Canonical Decisions

### D-001: Single semantic authority

- The only canonical execution pipeline is:
  - `Parser -> Resolved HIR -> Typed HIR -> HIR Evaluation`
- AST evaluation is retained only for:
  - explicit compatibility mode
  - differential/oracle testing
  - temporary bootstrap paths not yet migrated

### D-002: No implicit fallback

- `neve eval` and `neve run` must not silently fall back to AST.
- Any AST path must be explicitly requested and visible in output and tests.
- `neve check`, `REPL`, and `LSP` must not use AST fallback at all.

### D-003: One shared frontend driver

- CLI, REPL, and LSP must converge on one shared frontend/driver result.
- Consumers should not hand-roll `ModuleLoader + TypeChecker + diagnostics rewrite`.

### D-004: Typed HIR stays side-table based

- Do not rewrite all HIR nodes to carry embedded types in this migration.
- Define typed HIR via normalized side tables and semantic artifacts.

### D-005: Runtime convergence precedes new features

- Prioritize semantic consistency over feature expansion.
- Builtin/runtime behavior must converge under HIR before effectful capability growth.

### D-006: Module infrastructure splits before crate splits

- Decompose module infrastructure inside existing crates first.
- Do not extract new crates until boundaries are proven under CLI/REPL/LSP usage.
- `ModuleLoader` may temporarily survive as a compatibility facade only.

### D-007: Stable content identity replaces mtime as semantic authority

- Stable content hash becomes the semantic identity.
- `mtime`, dirty flags, and ad-hoc hash state are only performance hints.
- Invalidation must be phase-aware:
  - parse cache on source change
  - module graph on import-set / target change
  - lowering/name-resolution on own-source or dependency export-surface change
  - typed side tables on own-HIR or dependency type/export-surface change

### D-008: Std resolution and diagnostics attribution are single-owner concerns

- `std` module resolution must have one owner in the canonical pipeline.
- Cross-module diagnostics must carry:
  - owning module id
  - file/source identity
  - diagnostic payload
  - optional related module/file references

### D-009: Effects are explicit and runtime-mediated

- Pure expression evaluation must not directly trigger host-side effects.
- All host interaction must route through an effect runtime layer.
- The evaluator may construct effect values in pure mode, but cannot execute them there.

### D-010: Typed runtime objects replace string/record effect protocols

- Runtime objects must become first-class before syntax sugar is considered.
- Minimum first-class runtime objects:
  - `Path`
  - `Bytes`
  - `Command`
  - `ProcessResult`
  - `Pipeline`
  - `Redirect`
  - `Task<T>`

### D-011: Stdlib becomes layered around pure core and effect modules

- Stdlib layers should converge to:
  - pure core/data helpers
  - runtime object constructors/inspectors
  - effectful filesystem/process/network/task modules
  - higher-level platform/build helpers

### D-012: Process control is modeled, not inferred

- Timeout, cancellation, environment, cwd, stdin/stdout/stderr, tty, and signal behavior must be explicit in runtime objects/config.
- Process execution should distinguish:
  - command description
  - task handle / running process
  - completed result
- Full POSIX shell compatibility is out of scope for the initial effect runtime.

### D-013: Initial effect boundary is `Task<T>` plus effect summaries

- The first effect boundary should be typed task values plus explicit boundary operations such as await, poll, and cancel.
- Typed HIR should eventually carry effect-summary metadata so:
  - `check` and `LSP` can understand effectful programs without executing them
  - pure evaluation can reject execution at the boundary
  - effect runtime can drive only marked operations

### D-014: Pattern diagnostics become a first-class analysis phase

- Exhaustiveness and unreachable-pattern diagnostics must not remain ad-hoc logic embedded only inside expression inference.
- Pattern analysis should produce:
  - scrutinee shape classification
  - arm usefulness / reachability
  - missing-pattern set for diagnostics
- Initial hardening scope:
  - `Bool`
  - `Unit`
  - builtin `Option`
  - builtin `Result`
  - user enums

### D-015: Trait dispatch, associated types, and type display must share one semantic source

- Method dispatch, associated-type resolution, REPL `:type`, CLI diagnostics, and LSP hover must read from one canonical typed semantic artifact.

### D-016: Optionality and fallibility semantics are unified

- `match`, `?`, `??`, and safe field access must follow one coherent model for:
  - builtin `Option`
  - builtin `Result`
  - intentionally Option-like / Result-like user enums

### D-017: Type-checking pipeline splits into collect, infer, solve, finalize, diagnose

- Type checking should be refactored into explicit phases:
  - collect declarations and signatures
  - infer local constraints
  - solve unification and trait constraints
  - finalize normalized semantic artifacts
  - run secondary diagnostics
- `ConstraintSolver` and trait solving must join the main pipeline.

### D-018: Type-system hardening priority order

- Type-system hardening should proceed in this order:
  1. canonical typed semantic artifact shared by CLI / REPL / LSP
  2. trait method dispatch and associated-type use-site resolution unification
  3. canonical `Try` / `Option` / `Result` / coalesce / safe-field semantics
  4. dedicated exhaustiveness and usefulness analysis
  5. diagnostic normalization and documentation alignment

### D-019: New user-visible semantics require canonical-path parity before merge

- Do not merge new user-visible language behavior if it lands only in parser/unit tests or one command path.
- New observable semantics should meet all of the following before merge:
  - canonical HIR path covered by real end-to-end tests
  - CLI / REPL / LSP parity reviewed or explicitly gated
  - feature matrix / roadmap / spec updated if the behavior is user-visible
  - no expansion of AST fallback obligations beyond temporary compatibility needs

### D-020: PR-001 starts with cache/source extraction, not graph rewrites

- The first implementation batch for `PR-001` should extract incremental cache and parsed-source responsibilities out of `ModuleLoader` while preserving current external behavior.
- Do not combine that batch with import-resolution or module-graph rewrites.
- `ModuleLoader` remains the compatibility facade for now, but should delegate:
  - content hashing
  - parse cache reuse
  - dirty/mtime tracking
  - cache statistics

### D-021: PR-002 begins as a compatibility driver wrapper before final artifact redesign

- The first implementation batch for `PR-002` should centralize multi-module orchestration in `neve-frontend` without immediately locking in the final `TypedProgram` shape.
- It is acceptable for the first driver result to wrap:
  - `ModuleLoader`
  - per-module diagnostics
  - per-module method resolutions
  - shared visible type-name maps
- The goal of this batch is orchestration convergence, not final artifact perfection.
- CLI/LSP/REPL migration should only start after this compatibility driver exists and is covered by integration tests.

### D-022: PR-003 must land before LSP hover migration

- `neve-frontend` must own the canonical semantic side tables before `LSP` hover/semantic features migrate.
- `Document` should not switch from one local `TypeChecker` rerun to another hidden semantic recomputation path.
- The minimum canonical side tables for the first `LSP` migration batch are:
  - global types
  - global spans
  - local definition types
  - expression types
  - method resolutions
- Single-file frontend analysis should expose these side tables too, so `LSP` and future `REPL :type` work can share the same semantic source.

### D-023: Run/Eval AST paths must become explicit compat mode before deeper runtime convergence

- `neve run` and `neve eval` must not silently switch to AST when the HIR path cannot handle a source/module shape.
- The first migration batch should use an explicit CLI-facing compat switch such as `--compat-ast`.
- This batch should not redesign runtime semantics; it only makes backend selection explicit and testable.
- If `frontend/HIR` cannot handle a case and compat mode is not explicitly enabled, commands should fail with a clear rerun hint instead of silently changing backend.

### D-024: REPL should converge through a compatibility `FrontendSession` before deeper incremental redesign

- `PR-008` should first introduce a compatibility `FrontendSession` in `neve-frontend`.
- The first session scope is:
  - loaded-module graph ownership
  - persisted REPL module ownership
  - repeated type-check / diagnostics / type-name orchestration
- REPL runtime state should retain only:
  - evaluator/value persistence
  - binding/import bookkeeping required for execution
  - evaluated-module tracking
- Full source-db/incremental redesign can come later, but REPL should stop owning its own semantic pipeline first.

### D-025: `FrontendSession` should absorb REPL module-building orchestration, not only analysis

- After the first `FrontendSession` migration, `PR-008` should continue by moving:
  - current-module build/lower orchestration
  - imported module loading for one REPL input
  - import binding resolution for one REPL input
  into `neve-frontend`.
- `repl.rs` should not remain a second compatibility frontend around `Resolver + ModuleLoader`.
- It is acceptable for REPL runtime state to keep execution-oriented binding/import bookkeeping, but semantic construction should move behind `FrontendSession` APIs.

### D-026: New `FrontendSession` compatibility APIs should be validated in frontend-owned integration tests

- When `FrontendSession` gains compatibility APIs for REPL migration, those APIs should be covered in frontend integration tests rather than only indirectly through REPL command tests.
- Minimum direct coverage for the first batch:
  - build one in-memory module against loaded dependencies
  - resolve imported bindings and module aliases for one in-memory module
  - attribute diagnostics for newly loaded modules

### D-027: AST evaluator downgrade should start with a repo-wide `compat` namespace migration

- The first `PR-009` batch should introduce `neve_eval::compat::AstEvaluator` as the preferred internal path.
- Repository-internal AST evaluator consumers should migrate to the `compat` namespace before the legacy top-level re-export is removed.
- This batch should avoid semantic changes; it only narrows the public identity of the AST path and makes its compatibility role explicit in code and docs.

### D-028: Legacy top-level AST evaluator re-exports can remain temporarily, but should be hidden from primary docs after internal migration

- After internal consumers move to `neve_eval::compat`, the legacy top-level `AstEvaluator` / `AstEnv` re-exports may stay temporarily for compatibility.
- While they remain, they should be treated as legacy escape hatches rather than primary documented APIs.

### D-029: Runtime builtin convergence should target canonical frontend-visible entrypoints first

- `PR-004` should first converge HIR runtime behavior for builtin names already accepted by the canonical frontend/typeck pipeline.
- Expanding resolver/typeck visibility for additional builtin spellings must be a separate explicit step, not an incidental side effect of evaluator work.
- Full-pipeline tests for runtime convergence should therefore prefer entrypoints such as `std.list` module names before introducing new bare builtin-name expectations.

### D-030: Legacy top-level AST evaluator re-exports should be removed once repository-internal consumers finish migrating

- After repository-internal callers use `neve_eval::compat`, the hidden top-level `AstEvaluator` / `AstEnv` re-exports should be deleted rather than kept indefinitely.
- `neve-std` and other internal crates should treat `compat` as the only public AST-compat namespace.
- Any future AST-compat use must remain explicit at the import site as well as at the command/backend layer.

### D-031: Release cuts should follow coherent mainline milestones and exclude local AI-only files

- When a coherent mainline module lands and validation gates pass, it is acceptable to cut a new semver release instead of waiting for unrelated follow-up work.
- Release commits and tags must include only repository artifacts that belong to the product or its documentation.
- Local AI-assistance files such as `AGENTS.md` must remain outside staged release content.

## 7. Workstreams

## 7.1 WS-A: Canonical pipeline convergence

- Goal:
  - remove AST/HIR co-authority
  - make frontend driver the single orchestration entrypoint
- Primary crates:
  - `neve-frontend`
  - `neve-cli`
  - `neve-lsp`
  - `neve-eval`
- Completion signal:
  - `check`, `run`, `eval`, `REPL`, and `LSP` all consume one frontend semantic artifact

## 7.2 WS-B: Module infrastructure hardening

- Goal:
  - split source db, module graph, lowering context, cache store, and diagnostics attribution
- Primary crates:
  - `neve-hir`
  - `neve-frontend`
- Completion signal:
  - module identity is content-hash based
  - dependents tracking is phase-aware
  - CLI / REPL / LSP share one module graph

## 7.3 WS-C: Type-system hardening

- Goal:
  - move from "works on examples" to compiler-grade semantic closure
- Primary crates:
  - `neve-typeck`
  - `neve-frontend`
  - `neve-lsp`
  - `neve-cli`
- Completion signal:
  - one semantic artifact drives diagnostics, hover, `:type`, method dispatch, and pattern analysis

## 7.4 WS-D: Effect runtime and system automation model

- Goal:
  - establish explicit effect boundary and typed runtime objects
- Primary crates:
  - `neve-eval`
  - `neve-std`
  - `neve-typeck`
  - `neve-cli`
  - `neve-config`
  - `neve-store`
  - `neve-fetch`
- Completion signal:
  - no direct host effects from pure evaluation
  - system APIs are modeled via runtime objects and effect runtime APIs

## 7.5 WS-E: Documentation and test truthfulness

- Goal:
  - align docs and tests with real implementation status
- Primary files:
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
  - `docs/reference/spec.md`
  - `docs/reference/diagnostics.md`
  - `tests/*.rs`
- Completion signal:
  - no optimistic status claims remain for still-divergent semantics

## 8. Current Divergence Inventory

### 8.1 Explicit AST execution paths

- `neve-cli/src/commands/run.rs`
- `neve-cli/src/commands/eval.rs`
- `neve-cli/src/commands/build.rs`
- `crates/neve-config/src/flake.rs`
- `crates/neve-config/src/module.rs`
- `crates/neve-eval/src/ast_eval.rs`
- `crates/neve-std/src/lib.rs` via AST-only std overrides

### 8.2 HIR paths still bypassing a shared driver

- `neve-cli/src/commands/check.rs`
- `neve-cli/src/commands/repl.rs`
- `crates/neve-lsp/src/document.rs`
- `tests/module_typeck.rs`
- integration paths that still manually reconstruct parse/lower/typecheck/eval

### 8.3 Known semantic debt buckets

- module loading / name resolution / lowering / cache invalidation remain over-coupled
- tooling type display remains non-canonical
- runtime effects still leak through builtins
- type-system semantics are not yet fully normalized

## 9. Execution Roadmap

### PR-001: Split module infrastructure internally

- Separate source db, module graph, lowering context, cache store, and diagnostics attribution.
- Keep work inside existing crates first.

### PR-002: Introduce frontend driver

- Add shared multi-module driver in `crates/neve-frontend`.
- Introduce:
  - `TypedProgram`
  - `TypedModule`
  - `ModuleSemantics`
  - `FrontendSession`

### PR-003: Expose typed side tables from typeck

- Export read-only accessors for:
  - expression types
  - global types
  - global spans
  - local definition types
  - method resolutions

### PR-004: Converge HIR evaluator contract

- Make HIR evaluator consume typed driver output directly.
- Remove runtime behavior that still assumes AST evaluator ownership for builtin semantics.

### PR-005: Migrate `neve check`

- Replace hand-rolled multi-module orchestration with the frontend driver.

### PR-006: Migrate LSP

- Make `Document` cache driver output.
- Remove secondary `TypeChecker` pass from hover/semantic logic.

### PR-007: Migrate `neve run` and `neve eval`

- Default backend becomes canonical HIR only.
- Add explicit AST compatibility mode.
- Remove implicit AST fallback.

### PR-008: Migrate REPL

- Replace custom incremental semantic pipeline with `FrontendSession`.
- Keep only runtime/value persistence in REPL state.

### PR-009: Downgrade AST evaluator to compat/oracle

- Move AST evaluator to a compat namespace/module.
- Update tests and docs accordingly.

### PR-010: Migrate remaining AST-only platform paths

- `neve build`
- `neve-config::flake`
- `neve-config::module`

### PR-011: Introduce typed runtime objects and effect runtime skeleton

- Add builtin named types and runtime values for:
  - `Path`
  - `Bytes`
  - `Command`
  - `ProcessResult`
  - `Pipeline`
  - `Redirect`
  - `Task<T>`

### PR-012: Layer stdlib around runtime objects

- Move effectful stdlib functionality behind typed object modules and effect runtime entrypoints.
- Keep compatibility wrappers only as temporary adapters.

### PR-013: Migrate platform crates onto effect runtime

- `neve build`
- `neve-config`
- `neve-fetch`
- `neve-store`
- package/build orchestration paths

### PR-014: Harden typed semantic artifacts for tooling

- Expose normalized type/display artifacts from the frontend/typeck pipeline.
- Remove duplicate type recomputation from REPL and LSP.

### PR-015: Introduce dedicated pattern analysis and diagnostics pass

- Split exhaustiveness/usefulness analysis out of monolithic expression inference.
- Keep initial domain limited to `Bool`, `Unit`, builtin `Option/Result`, and user enums.

### PR-016: Unify trait method dispatch and associated-type use-site resolution

- Make method resolution and associated-type projection use one canonical solving path.
- Ensure runtime dispatch reads the same method resolution table produced by type checking.

### PR-017: Stabilize `Try` / `Option` / `Result` / coalesce / safe-field semantics

- Document one canonical model.
- Align type checking, evaluation, diagnostics, and tooling display around that model.

## 10. Required API Direction

## 10.1 Frontend

- `load_program(path, compat_mode) -> TypedProgram`
- `analyze_snippet(source, context) -> TypedProgram`
- `FrontendSession` for REPL/LSP/incremental consumers
- `AttributedDiagnosticSet` or equivalent driver-owned diagnostic output

## 10.2 Type checking

- expose normalized semantic artifacts instead of only ad-hoc checker methods
- include:
  - normalized global/local/expression types
  - method resolution table
  - associated-type projection resolutions
  - display-name map

## 10.3 Evaluation

- `eval_typed_program(&TypedProgram)`
- `eval_typed_module(&TypedModule)`
- effect-aware evaluation mode that distinguishes pure vs effect execution

## 10.4 Effect runtime

- explicit runtime entrypoints only
- host execution mediated through runtime traits / adapters
- task execution separated from task construction

## 10.5 Compat

- explicit AST compat entrypoints only
- no implicit backend selection in command code

## 11. Testing Strategy

## 11.1 Canonical-path tests

- Tooling and end-to-end tests must prefer the frontend driver.
- New integration tests must not manually reconstruct the semantic pipeline unless they are explicitly crate-local unit tests.

## 11.2 Differential compat tests

- AST/HIR parity tests belong in a dedicated differential suite.
- Compat tests must assert explicit AST backend selection.

## 11.3 Module infrastructure tests

- cover:
  - source identity and stable content hashing
  - import resolution
  - std resolution
  - dependents tracking
  - cache invalidation
  - diagnostics attribution

## 11.4 Type-system tests

- cover:
  - trait dispatch
  - associated-type use-site resolution
  - unified `Try` / `Option` / `Result` semantics
  - exhaustiveness/usefulness diagnostics
  - CLI/REPL/LSP type-display parity

## 11.5 Effect runtime tests

- cover:
  - runtime object construction
  - pure-mode rejection of effect execution
  - effect-mode execution of filesystem/process tasks
  - timeout, cancel, env, cwd, redirects, and result modeling

## 11.6 Regression gates

- `cargo test --workspace`
- `cargo test --test end_to_end -- --nocapture`
- `cargo clippy --workspace --all-targets -- -D warnings`

## 12. Documentation Sync Rules

- Every semantically meaningful PR must update whichever of the following are affected:
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
  - `docs/reference/spec.md`
  - `docs/reference/diagnostics.md`
  - user-facing tutorials or API docs when public behavior changes
- Feature matrix rows should not be marked complete just because parser support or AST compat exists.
- Public docs must reflect the canonical path, not the most permissive legacy path.

## 13. Merge / Review Gate

A change is aligned with the mainline only if it satisfies all applicable checks:

- pushes behavior toward the canonical pipeline rather than away from it
- reduces or at least does not expand AST/HIR semantic divergence
- improves or preserves CLI / REPL / LSP semantic consistency
- does not blur the pure/effect boundary
- is validated by real full-pipeline tests when user-visible semantics change
- updates roadmap / feature matrix / spec text when public behavior changes
- does not sneak in premature syntax or ecosystem commitments

If a change fails these checks, it should be treated as either:

- not ready for merge, or
- a consciously gated compatibility step with explicit follow-up debt recorded here

## 14. Compatibility and Rollback Policy

### Compatibility

- Preserve source compatibility where possible.
- Behavior changes are allowed where previous success depended on implicit fallback or non-canonical semantics.
- In those cases:
  - canonical mode must fail clearly
  - compatibility mode must be explicit

### Rollback

- Roll back per consumer migration, not by restoring implicit fallback.
- If a migration fails:
  - revert that consumer to the previous driver boundary
  - keep shared infrastructure that already proved correct
  - do not reintroduce hidden AST fallback

## 15. Success Metrics

Track progress with concrete metrics rather than narrative only:

- number of user-visible commands still using AST compatibility
- number of tooling paths still bypassing frontend driver
- number of module/cache decisions still based primarily on timestamps or unstable identity
- number of system APIs still encoded primarily as raw strings/records
- number of semantic doc sections still describing intended rather than actual behavior
- number of end-to-end programs executed through the canonical path

## 16. Update Protocol

- Append future direction changes under a new `D-xxx` entry.
- If workstream order changes, update `Execution Roadmap`.
- If scope, fallback policy, effect policy, or type-system priorities change, update `Canonical Decisions` first.
- When a user-visible feature is reviewed and rejected as premature, record that judgment here if it affects future merge decisions.

## 17. Progress Log

### 2026-04-13

- Started `PR-001` implementation with the first low-risk internal split.
- Extracted incremental cache and parsed-source storage out of `ModuleLoader` into `crates/neve-hir/src/incremental.rs`.
- Kept `ModuleLoader` as the compatibility facade and delegated:
  - dirty/mtime tracking
  - content hashing
  - parse cache reuse
  - cache statistics
- Continued `PR-001` with the second internal split.
- Extracted `ModulePath` and file/path resolution logic out of `ModuleLoader` into `crates/neve-hir/src/module_paths.rs`.
- Kept `ModuleLoader` public APIs stable while delegating:
  - relative-to-absolute module path normalization
  - std namespace file lookup
  - user module file lookup
- Continued `PR-001` with the third internal split.
- Extracted module graph bookkeeping out of `ModuleLoader` into `crates/neve-hir/src/module_graph.rs`.
- Kept `ModuleLoader` public APIs stable while delegating:
  - path/file-to-module lookup state
  - dependency edge tracking
  - reverse-dependent traversal
  - parent/child module relationships
- Continued `PR-001` with the fourth internal split.
- Extracted loader-owned diagnostics storage out of `ModuleLoader` into `crates/neve-hir/src/module_diagnostics.rs`.
- Kept `ModuleLoader` public APIs stable while delegating:
  - flat diagnostic accumulation
  - parse diagnostic forwarding
  - future cross-module attribution entrypoint
- Continued `PR-001` with the fifth internal split.
- Extracted import collection and module lowering/export-table assembly out of `ModuleLoader` into `crates/neve-hir/src/module_lowering.rs`.
- Kept `ModuleLoader` public APIs stable while delegating:
  - AST import extraction
  - `Resolver` orchestration for one module
  - export/re-export table assembly
- Started `PR-002` with a compatibility multi-module frontend driver.
- Added a first `FrontendDriver` / `ProgramAnalysis` wrapper in `crates/neve-frontend` around:
  - `ModuleLoader`
  - shared global signature collection
  - per-module diagnostics
  - per-module method resolutions
  - shared visible type-name maps
- Added integration coverage for:
  - multi-module driver success paths
  - parse-diagnostic preservation in imported modules
- Started `PR-005` by migrating `neve check` to the compatibility frontend driver.
- Removed `check`'s hand-rolled `ModuleLoader + TypeChecker + diagnostics rewrite` orchestration in favor of `FrontendDriver::analyze_module_path`.
- Kept command output behavior stable:
  - per-module parse diagnostics still emit against file contents
  - verbose parse/HIR counts still come from the canonical loaded modules
  - parse/type error counting still uses the same command-level rules
- Started `PR-003` by exposing canonical type-check side tables through `TypeChecker` and `neve-frontend`.
- Added read-only type-check accessors for:
  - global types
  - global spans
  - local definition types
  - expression types
- Added `ModuleSemantics` to single-file frontend analysis and compatibility driver output.
- Started `PR-006` by migrating `LSP` document hover/semantic logic to frontend-owned side tables.
- Removed the secondary `TypeChecker` pass from `crates/neve-lsp/src/document.rs`.
- Kept hover behavior stable while changing the data source to:
  - frontend global types
  - frontend local definition types
  - frontend expression types
  - frontend method resolutions
- Started `PR-007` by making `run/eval` AST execution paths explicit compat mode.
- Added `--compat-ast` to `neve run` and `neve eval`.
- Replaced implicit AST fallback with:
  - an explicit AST compat backend when requested
  - a clear rerun hint when HIR cannot handle the current std import/module shape
- Added command tests for:
  - rejecting implicit AST backend selection
  - succeeding with explicit AST compat on the same source shape
- Started `PR-008` with a compatibility `FrontendSession` in `crates/neve-frontend`.
- Moved REPL-owned semantic state toward frontend ownership by relocating:
  - loaded-module graph ownership
  - persisted REPL module ownership
  - repeated type-check / diagnostics / type-name orchestration
- Migrated `neve repl` to consume `FrontendSession` for:
  - current-module semantic analysis
  - loaded-module diagnostics
  - loaded-module method-resolution tables used before evaluation
  - `:type` name/type display based on frontend-owned semantic state
- Reduced REPL-local semantic duplication by removing direct `TypeChecker` orchestration from `repl.rs`.
- Validation completed with:
  - `cargo test -p neve-hir`
  - `cargo test --test module_loading`
  - `cargo test -p neve-frontend`
  - `cargo test --test frontend_driver --test frontend`
  - `cargo test -p neve`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test -p neve-typeck -p neve-frontend -p neve-lsp`
  - `cargo test --test frontend --test frontend_driver --test lsp`
  - `cargo test -p neve`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test -p neve repl_`
  - `cargo test --test end_to_end -- --nocapture`

### 2026-04-14

- Continued `PR-008` by moving REPL current-module construction behind `FrontendSession`.
- Added compatibility session APIs in `crates/neve-frontend/src/session.rs` for:
  - building one in-memory module against loaded dependencies
  - resolving imported bindings and module aliases for one in-memory module
  - reusing loaded-module diagnostics attribution through the same session
- Reduced `repl.rs` duplication further by removing direct command-level orchestration of:
  - `Resolver::new`
  - dependency module loading for one input
  - import binding resolution for one input
- Added frontend-owned integration coverage in `tests/frontend_session.rs` for:
  - building an in-memory module against a loaded dependency
  - resolving imported item bindings and namespace aliases
  - attributing diagnostics for newly loaded broken modules
- Validation completed with:
  - `cargo test --test frontend_session`
  - `cargo test -p neve`
  - `cargo test --test end_to_end -- --nocapture`
- Started the first `PR-009` batch by introducing repo-wide `neve_eval::compat::AstEvaluator` usage.
- Migrated repository-internal AST evaluator consumers to the `compat` namespace in:
  - `neve-cli`
  - `neve-config`
  - `tests/eval.rs`
  - `tests/end_to_end.rs`
- Kept the legacy top-level AST re-export temporarily for compatibility, but marked it as document-hidden.
- Updated `crates/neve-eval/src/lib.rs` docs so AST evaluation is described as explicit compatibility behavior rather than a peer execution mode.
- Validation completed with:
  - `cargo test -p neve-config`
  - `cargo test -p neve`
  - `cargo test --test eval --test end_to_end -- --nocapture`
- Continued `PR-004` by moving higher-order builtin ownership toward the HIR evaluator for canonical entrypoints already visible through the frontend.
- Recorded that builtin convergence work should target frontend/typeck-visible names first, with bare-name visibility expansion deferred to a separate explicit step.
- Narrowed the new higher-order runtime convergence tests to canonical `std.list` entrypoints so they validate the full frontend/typeck/eval pipeline instead of implicitly demanding new bare builtin visibility.
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test eval --test end_to_end -- --nocapture`
- Continued `PR-007` by migrating the canonical `neve run` HIR path from command-local `ModuleLoader + TypeChecker` orchestration to `FrontendDriver` / `ProgramAnalysis`.
- Kept explicit AST compat only for direct std-module execution and unsupported std-import shapes.
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test -p neve run_ -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
- Completed the next `PR-009` tightening step by removing the legacy top-level `AstEvaluator` / `AstEnv` re-exports from `neve-eval` after migrating remaining internal imports to `neve_eval::compat`.
- Updated `crates/neve-std` to import `compat::AstEnv` explicitly for stdlib AST-compat overrides.
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test -p neve-eval -p neve-std -p neve-config -p neve`
- Cut release preparation for the current coherent mainline milestone as `v1.2.0`.
- Updated workspace/package versioning and changelog entries for the driver/session convergence, explicit compat boundary, module-infrastructure split, and canonical CLI/runtime progress.
- Kept local AI-only files such as `AGENTS.md` outside staged release content.
- Validation completed with:
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
