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

### D-032: Release notes should describe canonical status truthfully, not the broadest legacy capability

- GitHub release notes should summarize the canonical mainline, explicit compatibility boundaries, current platform scope, and validation gates.
- Release notes should not imply that AST compatibility or partially migrated platform flows represent the primary supported architecture.
- Public release notes should avoid over-claiming effect-runtime completeness, shell compatibility, or fully migrated platform semantics.

### D-033: Evaluator callers should stop open-coding method-resolution injection

- As a transitional step toward `eval_typed_module`, `neve-eval` should expose helper APIs that accept semantic side tables directly enough to avoid repeated `set_method_resolutions + eval_module` sequences in commands and tests.
- This step should not reverse the architectural direction by making lower layers depend on frontend orchestration.
- Callers should prefer canonical semantic artifacts such as `ModuleSemantics.method_resolutions` over legacy duplicate fields where available.

### D-034: Post-v1.2.0 highest-priority work is finishing `neve eval` migration onto the shared frontend/driver path

- After `check`, `run`, `REPL`, and `LSP` have substantially converged, the next highest-priority mainline task is to remove the remaining split inside `neve eval`.
- `neve eval` still has two non-canonical execution shapes:
  - single-snippet evaluation via `analyze_ast`
  - import-bearing evaluation via temp-file/module-graph indirection
- The next implementation target should be a frontend-owned snippet/program API that lets `neve eval` consume the same canonical semantic artifact shape as the other core tools without hidden orchestration.
- This has higher priority than effect-runtime expansion and platform-crate migration because it closes the last obvious core-language CLI split in WS-A.

### D-035: `neve eval` snippet migration should use `FrontendSession`-backed in-memory module analysis, not synthetic temp modules

- The first `neve eval` convergence step should introduce a frontend-owned snippet analysis API built on `FrontendSession::build_module_from_ast` plus `analyze_module`.
- Local imports in `neve eval` should move to this in-memory frontend path instead of being forced through temp-file indirection.
- A narrow explicit compat branch may remain for std import shapes that the canonical frontend still cannot represent, but local-import support should no longer depend on the `run` command path.

### D-036: After `neve eval` snippet convergence, the next highest-priority semantic split is the remaining AST-only platform path

- Once `neve eval` no longer depends on `analyze_ast` or temp-module indirection for normal local-import snippets, the biggest remaining AST/HIR semantic authority split becomes `PR-010`.
- The next highest-priority migration target is therefore the remaining AST-only platform stack:
  - `neve build`
  - `neve-config::flake`
  - `neve-config::module`
- This should proceed before effect-runtime expansion because those paths still give AST evaluation a first-class product role instead of an explicit compat/bootstrap role.

### D-037: `neve-config::module` should keep its record-driven import graph for now, but must stop using AST evaluation as the per-file semantic authority

- `neve-config::module` imports are not language-level `import` statements; they are data in the evaluated module record (`imports = ["./base.neve"]`).
- Because of that, the first `PR-010` slice should not try to replace the config-module graph with the frontend/module-loader graph in one step.
- The mainline migration target for this slice is narrower:
  - keep config-owned recursive import loading based on evaluated `imports`
  - replace per-file `AstEvaluator` execution with canonical frontend/HIR analysis + HIR evaluation
  - let `neve-config` focus on translating the evaluated record into `Module` / `SystemConfig`
- Once this per-file semantic authority is moved to frontend/HIR, follow-up slices can decide whether config imports should later be modeled on top of typed runtime objects or a dedicated platform graph.

### D-038: The first `neve-config::flake` migration slice should move root flake evaluation and `outputs` closure invocation to frontend/HIR, without rewriting input resolution or lock semantics

- The highest-value semantic split in `neve-config::flake` is not the fetch/lock layer; it is the fact that `flake.neve` and `outputs` are still AST-evaluated.
- The next slice should therefore:
  - replace `Flake::load`/`Flake::parse` root evaluation with canonical frontend/HIR analysis
  - teach `eval_outputs` to invoke HIR closures explicitly instead of only `AstClosure`
  - keep path/git/archive input resolution, lock handling, and recursive input-flake loading behavior unchanged
- This keeps the migration aligned with the mainline goal of removing AST as product-path semantic authority, while avoiding an unnecessary rewrite of flake fetching/materialization in the same PR.

### D-039: Flake root evaluation may temporarily gate only on parse/frontend-load success, not all type diagnostics, until dynamic input-record typing is hardened

- Current flake `outputs = fn(inputs) ...` bodies routinely perform dynamic field access such as `inputs.dep.packages.x86_64_linux.default`.
- The HIR evaluator can execute these paths, but the current type system still reports record-field diagnostics for many of them because flake input records are not yet modeled precisely.
- For the first flake migration slice, the project should prefer:
  - canonical frontend parsing/lowering/module loading
  - canonical HIR evaluation
  - no hidden AST fallback
  - but a narrower diagnostic gate for flake root execution paths
- This is a transitional compromise only for `neve-config::flake`; it should be removed once typed runtime objects / flake input typing make these accesses statically representable.

### D-040: `neve build` should stop open-coding file parsing/evaluation and instead dispatch to canonical flake or frontend/HIR evaluation paths

- After `neve-config::module` and `neve-config::flake` move off AST-only root evaluation, the remaining product-path AST authority is `neve-cli/src/commands/build.rs`.
- The build command should split cleanly into two canonical entrypoints:
  - `flake.neve` and flake-root package selection should go through `neve-config::Flake`
  - plain `.neve` package files should go through frontend/HIR analysis plus HIR evaluation
- `build.rs` should keep derivation extraction, cache configuration, and builder execution locally, but it should stop owning parse/lower/eval orchestration.

### D-041: Record-related follow-up work should prioritize statically important access patterns, not open-ended record-system expansion

- The highest-priority record work is the set of record access patterns that still block or weaken canonical product paths today.
- Priority order:
  1. flake/config/build record access chains that currently require relaxed type gating
     - examples: `inputs.dep.packages.<system>.default`, derivation records, config/module reserved records
  2. unified field/safe-field/coalesce typing on `Record`, `Option<Record>`, and adjacent dynamic records
  3. consistent record type display and diagnostics across `check`, REPL `:type`, and LSP hover
  4. cross-module record-returning functions and imported record shapes in ordinary language code
- This work should aim at a compiler-grade closed loop for known product-path schemas and field-access behavior.
- It should not expand into general row polymorphism, open-record inference, structural subtyping, or new record syntax before the above product-path gaps are closed.

### D-042: Unknown-param record field chains should type-check without erasing known-record errors

- Product-path closures such as flake `outputs = fn(inputs) ...` require record field chains on values whose schema is only known dynamically at the call boundary.
- The immediate hardening step is:
  - allow `field` / `safe field` access to continue through `Var` / `Unknown` bases without emitting `non-record` errors
  - preserve existing hard errors for missing fields on known closed records
  - preserve safe-field's non-failing behavior on known records, including missing-field + `??` patterns
- This is a bounded product-path fix, not a commitment to general open-record typing.

### D-043: The next bounded record-hardening step is an inference-only dynamic-record type, not general row polymorphism

- Returning an unconstrained fresh type for `unknown.field` was enough to remove false negatives on product paths, but it loses too much information and cannot accumulate repeated field constraints on the same unknown base.
- The next step should therefore introduce an inference-only dynamic/open-record representation that can:
  - record required fields discovered from field access chains
  - merge repeated field requirements on the same unknown base
  - interoperate with closed record literals when enough structure becomes known
- This representation must remain internal to the compiler/typechecker:
  - no new surface syntax
  - no promise of user-visible row polymorphism
  - no structural subtyping feature claim in docs/reference

### D-044: `DynamicRecord` should accumulate field constraints across repeated access, unify against closed records, and surface consistently in tooling

- The first useful `DynamicRecord` slice is not a general open-record system; it is a bounded inference aid for canonical product paths and ordinary field chains on unknown parameters.
- This slice should:
  - bind repeated field accesses on the same unknown base to one accumulated `DynamicRecord`
  - allow nested chains such as `inputs.dep.packages.default` to build nested dynamic-record requirements
  - reject concrete call sites that fail to provide required fields once the unknown base is unified with a closed record literal
  - keep hard missing-field diagnostics for known closed records
- Tooling should render this internal type consistently as an open record shape such as `{ field: Ty, .. }` rather than exposing raw internal IDs differently across check, REPL, or frontend-derived displays.
- `safe field` should understand existing `DynamicRecord` shapes, but this slice still stops short of promising that unknown `base?.field` accepts both `Record` and `Option[Record]` with a fully precise static model.

### D-045: REPL semantic type display should delegate to frontend formatting wherever possible, and dynamic-record display parity should be tested through REPL and LSP

- After introducing `DynamicRecord`, keeping a third hand-written semantic type formatter inside the REPL would reintroduce a predictable divergence point.
- The next bounded consistency slice should therefore:
  - expose a frontend helper for formatting a type with an explicit visible-name map
  - have REPL semantic type rendering delegate to that helper for ordinary language types
  - keep only REPL-specific pseudo-types such as `Fn` / `Lazy[...]` as local formatting exceptions
- LSP already routes semantic hover formatting through frontend; this slice should add regression coverage proving that dynamic-record shapes are rendered readably and consistently in both LSP hovers and REPL `:type`.

### D-046: Unknown-base safe-field access should carry an internal record-or-option-record constraint, so obvious non-record callsites fail statically

- After `DynamicRecord`, the remaining high-value unsoundness on the record sub-line was `config?.name ?? "default"` still accepting concrete non-record callsites such as `readName(42)`.
- The next bounded fix should therefore introduce an internal safe-field base constraint that:
  - applies when `?.field` is used on an unknown base or unknown option payload
  - accepts later unification with a concrete `Record`, `DynamicRecord`, or builtin `Option[...]` payload thereof
  - allows missing fields, because `?.field` may legitimately produce `None`
  - still checks field-type consistency when the field is present
- This must remain an internal compiler aid:
  - no new surface syntax
  - no promise of unions or general “record or option” types in the language
  - user-visible output may describe it readably, but docs/reference should not present it as a stable source-level type feature

### D-047: The next type-system priority on the record/option sub-line is to unify `try`, `coalesce`, and `safe field` around one optional-flow model

- After `DynamicRecord` and `SafeRecordBase`, the next highest-value gap is no longer “record access on unknown bases”; it is that `?`, `??`, and `?.` still rely on adjacent but not fully unified type rules.
- The next bounded slice should:
  - extract one internal notion of “optional-like payload extraction” for builtin `Option`, builtin `Result`, supported enum-like shapes, and `safe field`
  - make `coalesce_result_type`, `try_result_type`, and `safe_field_result_type` share the same normalization and diagnostic staging where the semantics overlap
  - ensure the type checker rejects the same obviously invalid callsites that runtime would reject, while still preserving the intended non-failing `?.field` missing-field behavior
- This slice should stay narrower than full algebraic effect/union typing:
  - no new user-visible type forms
  - no broad promise of arbitrary enum structural compatibility
  - no attempt to solve all row-polymorphism or open-record propagation in the same PR
- Expected first implementation boundary:
  - central helper(s) in `crates/neve-typeck/src/check.rs`
  - no HIR or syntax expansion
  - regression matrix across typeck, frontend diagnostics, LSP hover, and end-to-end runtime parity for `?`, `??`, and `?.`

### D-048: After the first optional-flow convergence slice, the project should prefer “close semantic loops” over introducing more inference-only helper types

- After `DynamicRecord`, `SafeRecordBase`, and the first shared optional-flow helper, the next step should not automatically be “add another internal constraint type for unknown-base `?` or `??`”.
- The preferred sequencing from here is:
  1. close the existing optional-flow semantic loop
     - align runtime and typeck expectations for `?`, `??`, and `?.`
     - make diagnostics/messages consistent across CLI, REPL, and LSP
     - fill missing test matrix cells before increasing inference complexity
  2. harden associated nearby semantics that directly affect product paths
     - match exhaustiveness / unreachable pattern quality on builtin `Option`/`Result`
     - trait method dispatch and associated-type use-site stability where optional-flow values appear
  3. only then consider whether unknown-base `?` / `??` need their own bounded internal constraint
- The bar for step 3 is explicit:
  - a recurring false-positive or false-negative on real product-path code
  - not just “the type system could theoretically infer more”
- This ordering fits the project’s design philosophy:
  - canonical pipeline convergence before inference ambition
  - bounded internal mechanisms before generalized polymorphism
  - real-path closure before abstract completeness

### D-049: The next planning unit should be a bounded “optional-flow closure matrix”, not another broad refactor

- The next design/implementation unit on this sub-line should be framed as a testable matrix with a clear exit criterion.
- Recommended matrix rows:
  - builtin `Option` with `?`
  - builtin `Result` with `?`
  - enum-like `Some/None` with `?` and `??`
  - enum-like `Ok/Err` with `?`
  - `record?.field`
  - `option(record)?.field`
  - invalid known non-optional uses of `?` and `??`
- Recommended matrix columns:
  - typeck acceptance / rejection
  - frontend diagnostic shape
  - REPL `:type`
  - LSP hover
  - end-to-end runtime parity where evaluation is supposed to succeed
- Completion signal for this unit:
  - no row relies on a bespoke rule that is not represented in the shared optional-flow helper story
  - user-facing tools render and diagnose the same class of optional-flow program consistently

### D-050: Once canonical typeck rejects known-invalid `?` / `??`, both HIR evaluation and AST compat must reject the same inputs instead of passing them through

- The project should not preserve legacy evaluator behavior where:
  - `41?` evaluates to `41`
  - `41 ?? 0` evaluates to `41`
  - unrelated enum values flow through `?` / `??` unchanged
- After the first shared optional-flow helper slice, evaluator behavior for these known-invalid cases should converge with canonical typeck:
  - HIR evaluation must raise an explicit runtime type error
  - AST compat should mirror the same behavior for the shared subset used in differential/oracle tests
- This slice is intentionally narrow:
  - no new inference-only helper type for unknown-base `?` / `??`
  - no expansion of optional-flow surface semantics
  - no claim that AST compat becomes canonical again; it only stops disagreeing on invalid inputs

### D-051: `PR-015` should land as an extracted internal pattern-analysis layer before any broader solver or HIR redesign

- The current `check_match_coverage` logic in `crates/neve-typeck/src/check.rs` is already good enough to define the initial support domain, but it is in the wrong place and too weak for subset-shadowing/usefulness diagnostics.
- The first `PR-015` implementation slice should therefore:
  - extract coverage/usefulness logic into a dedicated internal module inside `neve-typeck`
  - keep the initial semantic domain unchanged:
    - `Bool`
    - `Unit`
    - builtin `Option`
    - builtin `Result`
    - user enums
  - keep the existing “after arm type checking, before final match result publication” call-site position
- This slice should not be coupled to:
  - a full type-check pipeline rewrite
  - a new HIR form
  - effect-system work
  - generalized algebraic pattern compilation

### D-052: The first dedicated pattern-analysis API should compute usefulness and exhaustiveness from normalized typed shapes, with conservative guard handling

- The first extracted pattern-analysis layer should consume:
  - normalized scrutinee type after substitution
  - the typed `MatchArm` list
  - enum/builtin constructor metadata already available in `TypeChecker`
- The first extracted layer should produce one internal result struct carrying:
  - `missing_patterns: Vec<String>`
  - `unreachable_arms: Vec<UnreachableArm>`
  - `coverage_complete_at: Option<Span>`
  - optional per-arm usefulness classification for future diagnostics/tests
- Guard policy for the first extracted slice should remain conservative:
  - guarded arms do not contribute to exhaustiveness
  - guarded arms are not used to prove later arms unreachable
  - this policy should be explicit in tests/docs instead of being an accidental side effect of skipping guards in loops
- The first usefulness hardening target beyond today's behavior is subset-shadowing inside already-supported domains:
  - `Some(_)` before `Some(1)` should mark the latter unreachable
  - `Ok(_)` before `Ok(x)` should mark the latter unreachable
  - `true | false` before any later bool arm should mark the later arm unreachable
  - single-variant enums and irrefutable constructor patterns should participate consistently in this check

### D-053: The first usefulness slice should treat “possible match space” separately from “coverage space”

- Exhaustiveness and usefulness should not be forced through one boolean notion of “covers variant”.
- The first extracted usefulness slice should therefore keep two separate internal questions:
  - does this arm fully cover a domain fragment for exhaustiveness?
  - can this arm still possibly match any not-yet-covered fragment?
- This separation is what allows the bounded first-wave diagnostics that matter on the mainline:
  - `Some(_)` making a later `Some(1)` unreachable
  - `Ok(_)` making a later `Ok(value)` unreachable
  - a fully enumerating bool or-pattern making later bool arms unreachable
- This slice still remains intentionally bounded:
  - no generalized constructor-space algebra
  - no full matrix decomposition of tuple/record payload patterns
  - no expansion beyond the already-supported pattern domains

### D-054: The next `PR-015` slice should improve usefulness provenance before expanding pattern domains

- After the first subset-shadowing slice, the next highest-value gap is diagnostic precision rather than semantic breadth.
- Current unreachable-pattern reporting can already detect the right class of cases, but it still relies on a coarse `shadowed_by` source in some subset-shadowing paths.
- Before adding any broader pattern domains, the next slice should make usefulness results carry explicit provenance:
  - which earlier arm witnesses the redundancy
  - whether redundancy came from full coverage or subset-shadowing
  - whether the arm was skipped from usefulness reasoning because it had a guard
- This should land before any attempt to expand into tuple/record-pattern usefulness or broader constructor-space algebra.

### D-055: `PatternAnalysisResult` should grow per-arm usefulness state, not just final diagnostics-ready vectors

- The current extracted layer is already the right home for this information; the next step is to make that structure precise enough that diagnostics are derived from analysis rather than reconstructed from ad-hoc fallbacks.
- The preferred next internal shape is:
  - `arm_usefulness: Vec<ArmUsefulness>`
  - where `ArmUsefulness` can distinguish at least:
    - `Useful`
    - `Redundant { witness_span, reason }`
    - `GuardedIgnored`
- `reason` should remain internal and bounded for now:
  - `CoveredByPreviousArms`
  - `SubsetShadowed`
  - optional future variants for domain-specific refinement
- This does not change the user-facing diagnostic contract yet; it just prevents the next usefulness slices from encoding provenance in incidental control flow.

### D-056: After provenance lands, the next `PR-015` priority is to consume usefulness reason in diagnostics before expanding the analyzed pattern domain

- Once `PatternAnalysisResult` records per-arm usefulness and redundancy reason, the next highest-value step is to use that information in diagnostics instead of leaving it as dead internal metadata.
- The next slice should refine unreachable-pattern diagnostics so they can distinguish at least:
  - full coverage by previous arms
  - subset-shadowing by a narrower earlier arm
- The immediate goal is not prettier wording for its own sake; it is to make diagnostics semantically truthful before the analysis grows more complex.
- This should happen before any attempt to extend usefulness/exhaustiveness to tuple, record, or list-rest pattern domains.

### D-057: Pattern diagnostics should become deterministic before the project broadens pattern-domain support

- Current enum-pattern analysis still relies on `EnumInfo` data stored in hash-map form, which means missing-pattern ordering and some witness selection can drift in ways that are semantically harmless but user-visible.
- Before expanding the pattern-analysis domain, the project should preserve declaration-order information for enum variants and use it for:
  - missing-pattern formatting
  - deterministic witness selection when multiple earlier arms cover different variants
  - future docs/tests that need stable output
- This does not require a broad type-system change; it is a local data-shape hardening step inside type-check/pattern-analysis metadata.

### D-058: Tuple / record / list-rest pattern analysis should remain explicitly deferred until diagnostics fidelity and deterministic ordering are complete

- Neve already supports tuple, record, and list-rest patterns in matching/type checking, but their exhaustiveness/usefulness story is still incomplete.
- The project should not “just expand the domain” next, because that would multiply diagnostic-quality and determinism problems that are only now getting isolated.
- The gating condition for any broader domain expansion under `PR-015` is:
  - usefulness reason is actually reflected in diagnostics
  - enum-domain output is deterministic
  - the supported-domain regression matrix remains green
- Only after those conditions hold should the project decide whether tuple/record/list-rest pattern coverage belongs in a later `PR-015` slice or in a separate follow-up workstream.

### D-059: The next concrete `PR-015` execution order is diagnostics truthfulness first, deterministic enum ordering second, domain-expansion evaluation third

- The next three `PR-015` steps should execute in this order:
  1. consume `RedundancyReason` in unreachable-pattern diagnostics
  2. preserve enum declaration order in pattern-analysis metadata and formatting
  3. only then re-evaluate whether tuple/record/list-rest patterns are the right next domain
- Step 1 should remain local to:
  - `crates/neve-typeck/src/check.rs`
  - `crates/neve-typeck/src/errors.rs`
  - regression tests that assert diagnostic labels/notes/help
- Step 2 should remain local to:
  - enum collection metadata in `TypeChecker`
  - `pattern_analysis` missing-pattern / witness formatting
  - regression tests that assert stable missing-pattern ordering
- Step 3 is explicitly a planning checkpoint, not an automatic implementation follow-up.

### D-060: The next tests for `PR-015` should assert diagnostic semantics, not just warning presence

- The supported-domain matrix is now strong enough that the next failures are more likely to be “wrong reason” or “unstable output” than “warning missing entirely”.
- The next `PR-015` test additions should therefore focus on:
  - label text for `CoveredByPreviousArms` vs `SubsetShadowed`
  - previous-label witness span selecting the intended earlier arm
  - deterministic `missing patterns: ...` note ordering for enums and builtin `Option` / `Result`
- This is still canonical-path work:
  - typeck integration tests remain the primary harness
  - frontend/end-to-end tests should only lock one or two representative warning/error surfaces, not duplicate the whole matrix

### D-061: The first `PR-016` slice should canonicalize trait-impl method signatures inside the trait resolver before expanding associated-type semantic surfaces

- The current mainline gap is not “missing syntax” or “missing trait registry”, but that `TraitResolver` still stores raw impl method signatures while method-call inference/runtime dispatch already rely on the resolver as a canonical lookup path.
- This creates future-refactor pressure around associated-type returns:
  - impl methods can be checked against concrete `Self` / `Self.Item` substitutions
  - but method-call inference may still read the raw, pre-normalized impl signature from the resolver
  - tooling and runtime would then only stay aligned by accident or by later body-check side effects
- The first bounded `PR-016` slice should therefore:
  - canonicalize trait-impl associated-type bindings and method signatures during the pre-body impl-signature pass
  - keep the change scoped to trait impls only
  - continue to expose the result through the existing semantic artifacts (`expr_types`, `local_types`, `global_types`, `method_resolutions`) instead of introducing a larger new projection table immediately
- Inherent impl canonicalization, broader associated-type projection artifacts, and deeper trait-solver redesign remain follow-up work, not part of this slice.

### D-062: The next bounded `PR-016` slice should expose explicit associated-type projection resolutions through the canonical semantic artifact before any wider tooling surface expansion

- After trait-impl method signatures are canonicalized, the next remaining mainline gap is that explicit `Self.Item` use-sites are still only normalized implicitly during checking.
- This is good enough for some current behaviors, but it leaves future tooling/refactor work under-specified:
  - frontend results do not yet carry a first-class record of which explicit type-use spans resolved to which concrete associated types
  - LSP / REPL / future diagnostics would otherwise need to re-derive that information indirectly from already-normalized signatures or body inference
- The next bounded slice should therefore:
  - record `SelfAssoc` resolution at explicit type-use spans inside `TypeChecker`
  - expose those resolutions through `ModuleSemantics`
  - clear that per-module table in session-style incremental analysis just like method resolutions
- This slice should remain deliberately narrow:
  - no broader solver redesign
  - no new language surface
  - no expansion to inherent impl canonicalization
  - no new user-facing hover or syntax commitments beyond exposing the canonical semantic artifact itself

### D-063: After projection exposure lands, the next `PR-016` priority is consumer adoption of that canonical artifact, not more trait-system breadth

- Once explicit associated-type projections are recorded in `ModuleSemantics`, the highest-value next step is to make consumer layers read that artifact directly where explicit type-use spans matter.
- The point of this slice is not to make the trait system “more powerful”; it is to stop leaving canonical information unused after type checking already computed it.
- The next consumer-adoption slice should therefore focus on:
  - frontend-owned helpers for formatting or looking up explicit type-use spans via `ModuleSemantics`
  - LSP hover / symbol-facing surfaces that currently only show source-level `Self.Item` shape or re-derived type strings
  - diagnostics or notes that can now point to the concrete projected type without re-solving
- This remains a narrow, mainline-aligned slice:
  - no new syntax
  - no solver expansion
  - no inherent-impl generalization
  - no broad UI redesign

### D-064: The next planning boundary after consumer adoption is dispatch/assoc diagnostic fidelity, not immediate solver redesign

- After tooling and diagnostics consume explicit projection artifacts directly, the next likely `PR-016` checkpoint should be about diagnostic truthfulness:
  - missing impl / missing method / assoc-type mismatch notes should be sourced from the same canonical dispatch/projection information
  - method dispatch and associated-type use-site errors should no longer depend on partially duplicated formatting logic
- Only after those consumer and diagnostic slices are complete should the project reconsider broader trait-solver work, inherent-impl canonicalization, or richer associated-type projection models.

### D-065: The first diagnostic consumer of canonical projection artifacts should be impl-signature mismatch reporting, not a wider diagnostic sweep

- The narrowest high-value diagnostic surface after consumer adoption is the existing:
  - `impl method ... does not match trait ... signature`
  path.
- This path already computes canonical expected/actual signatures, but it still benefits from explicit projection-aware labels:
  - users can now see where `Self.Item` resolved to a concrete type in the mismatching signature
  - the diagnostic can point to canonical projection results without broadening to unrelated trait errors in the same slice
- The first diagnostic slice should therefore:
  - attach projection-aware labels to impl-signature mismatch diagnostics
  - source those labels from the canonical projection table rather than re-solving
  - remain local to `neve-typeck` / frontend diagnostic flow
- Broader diagnostic adoption (missing impls, assoc bound failures, dispatch ambiguity) remains follow-up work.

### D-066: After impl-signature mismatch reporting adopts projection labels, the next bounded diagnostic slice should cover impl-body return mismatches before any broader trait-error sweep

- The next closest user-visible gap after impl-signature mismatch is the existing:
  - `impl method ... return type`
  diagnostic emitted from impl-body checking.
- This path already goes through canonical type checking and now records projection spans, so it is the most natural follow-up:
  - users can see the concrete projected type that `Self.Item` stands for when the body returns the wrong type
  - the change remains narrowly local to impl-body checking rather than broadening to all trait diagnostics at once
- This slice should:
  - attach projection-aware labels to impl-body return mismatch diagnostics
  - reuse the same canonical projection artifact and helper path as the impl-signature mismatch slice
  - stop before expanding to missing methods, associated-type bounds, or dispatch-ambiguity diagnostics

### D-067: Before any broader `PR-016` consumer or diagnostic expansion, the canonical projection artifact must stop polluting trait-declaration spans with impl-specific resolutions

- After the first projection-consumer slices landed, the next highest-value task is not wider trait tooling coverage but boundary correction:
  - explicit projection recording currently risks attaching impl-specific concrete resolutions to trait declaration `Self.Item` spans when trait and impl live in the same module
  - that would violate the canonical artifact boundary because trait definitions should stay source-level unless a concrete use-site is actually being resolved
- The next bounded `PR-016` slice should therefore:
  - restrict projection recording to impl-side and concrete use-site contexts
  - preserve trait declaration type-use spans as source-level `Self.Item` in shared semantic artifacts
  - add focused frontend/LSP regressions that exercise a trait and impl in the same module
- Broader diagnostic adoption or further projection-surface expansion should wait until this boundary is correct.

### D-068: After the projection-boundary correction, the next narrow `PR-016` slice should canonicalize assoc-type bound diagnostics before touching method-call failure semantics

- Once the shared projection artifact boundary is correct, the next highest-value remaining trait/assoc gap is still on the impl side:
  - `check_assoc_type_bounds` currently diagnoses against the raw impl-side associated type syntax
  - that means the error path can lag behind the canonicalized impl-signature / assoc-binding view already used elsewhere in `PR-016`
- The next bounded slice should therefore:
  - make assoc-type bound checking and its diagnostics consume the same canonical impl assoc-type bindings already computed for impl-signature normalization
  - prefer concrete/canonical associated-type renderings in labels and notes where available
  - stay local to impl-side trait/assoc diagnostics rather than broadening to method-call failure, dispatch ambiguity, or missing-method runtime behavior
- Method-call failure semantics remain follow-up work because they are tied to the existing fallback-to-call behavior and carry a wider user-visible blast radius than this impl-side diagnostic hardening slice.

### D-069: The assoc-bound slice should first factor a local canonical impl-assoc binding helper, not reorder the whole impl-check pipeline

- Inspection of the current `PR-016` code shows that `impl_signature_assoc_types` is still shallow:
  - it collects raw impl/default associated-type values
  - it does not recursively normalize `Self.OtherAssoc` chains inside those bindings
- Reordering `check_all_impls` to canonicalize the resolver earlier would be a wider semantic move than needed for the next slice.
- The preferred next step is therefore:
  - factor a local helper that resolves impl associated-type bindings to canonical concrete forms for the current impl
  - give that helper a narrow cycle guard / visited-set so bad self-recursive assoc definitions fail locally instead of looping
  - reuse that helper first in assoc-bound diagnostics and then, if it proves correct, consider collapsing the older shallow `impl_signature_assoc_types` path onto it
- This keeps the next change:
  - local to impl-side canonicalization
  - testable with a small number of assoc-bound regressions
  - lower risk than reordering the whole impl validation pipeline

### D-070: After the local canonical assoc-binding helper proves itself on assoc-bound diagnostics, `PR-016` should collapse the older shallow impl-signature assoc path onto that helper before any method-call failure work

- The assoc-bound slice demonstrated that the local helper can correctly:
  - resolve `Self.OtherAssoc` chains
  - honor defaults
  - substitute `Self`
  - stop on local cycles
- The next highest-value mainline cleanup is therefore to stop maintaining two different impl-assoc views:
  - the newer canonical helper used by assoc-bound diagnostics
  - the older shallow `impl_signature_assoc_types` path still used by impl-signature normalization and resolver canonicalization
- The next bounded slice should:
  - compute one canonical impl-assoc map per impl inside the existing `check_all_impls` flow
  - reuse that map for impl-signature checking and `TraitResolver` normalization
  - add regressions around trait default associated types / alias chains affecting method return types and tooling-visible results
- Method-call failure semantics remain deferred until the canonical resolver itself no longer lags on default/alias assoc bindings.

### D-071: After impl-side canonicalization converges, the next `PR-016` step should first codify method-call dispatch precedence before attempting any breaking failure-semantics change

- Current evidence shows that method syntax already has a real user-visible fallback model:
  - type checking falls back from unresolved method dispatch to ordinary callable-target checking
  - HIR evaluation falls back from missing `method_resolutions` to evaluating the lowered target callable
  - tests already rely on this with examples like `21.twice()`
- That means the next mainline move should not be to silently remove fallback first.
- The next bounded slice should therefore:
  - define and lock one canonical dispatch order for `x.f(y)`:
    1. inherent / trait method resolution on the receiver
    2. if unresolved, ordinary callable-target fallback using the lowered `f(x, y)`-style target
  - add full-pipeline tests that cover:
    - resolved trait/inherent method dispatch
    - callable-target fallback
    - no-method and no-callable failure paths
    - type-display / hover behavior on both branches
  - update spec / roadmap / feature-matrix language so the dispatch model is explicit instead of accidental
- Only after that model is explicit and test-locked should the project decide whether any branch deserves a dedicated missing-method diagnostic or a future breaking change.

### D-072: The first method-call semantics slice should lock the current lowered-target failure path as an explicit temporary behavior, not silently change it

- Once dispatch precedence is made explicit, unresolved method calls with no callable fallback still surface through the lowered target path today.
- Current canonical behavior is therefore:
  - method dispatch first
  - callable-target fallback second
  - if both fail, diagnostics currently come from the lowered target path rather than a dedicated missing-method error
- The next slice should test-lock that current behavior rather than changing it implicitly.
- A dedicated missing-method diagnostic remains follow-up work and should only be introduced as an explicit design decision after the precedence model is stable in docs/tests.

### D-073: After dispatch precedence is explicit, unresolved method calls with no callable fallback should move to a dedicated call-site diagnostic

- Once the dispatch order is explicit and test-locked, leaving the no-method / no-callable branch on the generic lowered-target `undefined global` diagnostic becomes misleading.
- The next bounded slice should therefore change only this narrow branch:
  - method dispatch still runs first
  - callable-target fallback still runs second
  - if no method exists and the lowered callable target is unresolved, emit a dedicated method-call diagnostic at the original call site
  - if a callable target exists, keep ordinary callable-target diagnostics unchanged
- This slice should:
  - add a stable diagnostic code for unresolved method calls on the receiver type
  - cover typeck/frontend/LSP/end-to-end behavior with focused tests
  - update spec / diagnostics / roadmap / feature-matrix text so the failure story matches the real pipeline
- This is still not a decision about the long-term removal or retention of callable fallback as a language feature; it only canonicalizes the current failure branch.

### D-074: The next `PR-017` optional-flow matrix slice should close tooling parity with valid imported source shapes, not by broadening implicit builtin scope

- After typeck/runtime alignment for `?`, `??`, and `?.`, the next highest-value gap is tooling parity:
  - REPL `:type`
  - LSP hover
- The bounded goal is to prove these tools show the same resulting types as the canonical frontend/typeck path for valid optional-flow programs.
- This slice should stay disciplined about scope:
  - use explicit `import std.option as option` / `import std.result as result` when those namespaces are needed
  - do not broaden REPL or LSP default scope just to make tests pass
  - treat differences caused by missing imports as scope-model issues, not optional-flow semantics
- The preferred first regression rows are:
  - imported `option.some(41)? + 1`
  - imported `option.none ?? 5`
  - imported `option.some(record)?.field ?? default`
- This keeps the closure matrix aligned with canonical source semantics instead of introducing a second, more permissive tooling-only surface.

### D-075: After valid-shape tooling parity lands, the next `PR-017` slice should lock invalid optional-flow diagnostics across frontend, REPL, and LSP before any new inference work

- The remaining high-value gap in the optional-flow closure matrix is no longer successful type display; it is diagnostic parity for invalid uses.
- The next bounded slice should therefore focus on source shapes such as:
  - `41?`
  - `41 ?? 0`
  - safe-field callsites that violate the current record-or-option-record boundary
- The goal is not to invent new errors or broaden the accepted language.
- The goal is to ensure the same canonical frontend/typeck diagnosis is surfaced consistently through:
  - frontend diagnostics
  - REPL `:type` diagnostics
  - LSP document diagnostics
- This slice should remain narrow:
  - prefer adding parity tests first
  - only change formatting/rewriting layers if those tests expose a real mismatch
  - do not introduce new inference-only helper types for unknown-base `?` / `??`
  - do not broaden implicit builtin scope to make invalid-shape tests pass
- Completion signal for this slice:
  - invalid optional-flow programs report the same core message/code class through frontend, REPL, and LSP
  - the project can then treat the optional-flow closure matrix as sufficiently closed and move on to the next bounded mainline topic

### D-076: The first `PR-011` slice should introduce dormant runtime-object identities before migrating stdlib contracts

- After `PR-017` reaches a practical closure point, the next highest-value mainline move is to begin `PR-011`.
- The first slice should remain intentionally narrow:
  - add builtin named-type identities for the planned runtime-object family
  - introduce internal runtime values for the simplest non-effectful objects first (`Path`, `Bytes`)
  - make core display/introspection layers aware of those values (`typeOf`, REPL runtime-value typing/formatting)
- This slice should *not* yet:
  - change path/io/fetch stdlib function contracts
  - introduce effect execution APIs
  - change path literal typing
  - promise user-visible `Command` / `Task` behavior before the value/type skeleton exists
- The point of this first slice is to establish stable internal object identity so later stdlib/runtime migrations stop depending on raw `String`/`Record` as the only representation.

### D-077: The next `PR-011` slice should expose the smallest user-visible `Path` bridge before any broad stdlib migration

- After dormant `Path` / `Bytes` runtime-object identity exists internally, the next highest-value move is to make `Path` reachable from canonical user code without yet rewriting the old string-based system APIs.
- This slice should stay intentionally narrow:
  - add an explicit constructor bridge in `std.path`, `path.fromString : String -> Path`
  - rely on the existing generic `toString` / `typeOf` surface for basic display and introspection
  - keep the older string-based `path.join` / `path.parent` / `path.filename` / `path.extension` / `path.is_absolute` contracts unchanged for now
- This slice should *not* yet:
  - migrate `io.*` or `fetch.*` to `Path`
  - change path literal typing
  - expose a full `Bytes` stdlib surface before a dedicated module contract is designed
- The point of this slice is to make `Path` a real canonical runtime value in user programs first, while deferring the broader stdlib contract migration to `PR-012`.

### D-078: The first `PR-012` slice should add only minimal typed-path compatibility adapters, not a full stdlib path migration

- After `path.fromString` exposes `Path` to user code, the next mainline step should make `Path` minimally compositional before touching `io` / `fetch` / path literals.
- This slice should stay intentionally narrow:
  - add a small typed-path adapter set in `std.path`, enough to compose and inspect `Path` values without immediately converting back to `String`
  - keep all existing string-based `std.path` functions available and unchanged
  - prefer consumer/tooling parity tests over a broad stdlib expansion
- The first adapter set should be limited to:
  - `path.joinPath : Path -> String -> Path`
  - `path.parentPath : Path -> Option[Path]`
  - `path.isAbsolutePath : Path -> Bool`
- This slice should *not* yet:
  - migrate `io.*` or `fetch.*` to `Path`
  - add a full typed `Bytes` stdlib surface
  - change path literal typing
- The point of this slice is to make `Path` useful enough inside canonical user code that later `PR-012` / `PR-013` work can build on a real typed path flow instead of string round-tripping.

### D-079: Prompting for mainline work should follow one dependency-ordered chain, not a bag of parallel templates

- For Neve mainline work, prompts should be written as a sequence where each prompt consumes the conclusion of the previous one.
- The default ordered chain should be:
  1. current-state sync
  2. next-priority decision
  3. bounded design
  4. file-level implementation plan
  5. execution
  6. parity/regression validation
  7. documentation synchronization
  8. release/merge gate decision
- Each prompt in the chain should:
  - restate the accepted conclusion from the previous step
  - narrow the next decision or implementation surface
  - explicitly list what remains out of scope for that step
- This process is preferred over broad "optimize the project" prompts because the mainline priority is controlled semantic convergence, not free-form expansion.

### D-080: The next `PR-012` slice should bridge exactly one minimal `io.*` host boundary to `Path`

- After `path.fromString` and the first typed-path adapters exist, the next highest-value move is to prove that a typed runtime object can cross one real host boundary without broad stdlib migration.
- This slice should stay intentionally narrow:
  - add `io.readFilePath : Path -> String`
  - keep `io.readFile : String -> String` unchanged
  - treat parity across stdlib runtime, canonical type checking, frontend, HIR evaluation, end-to-end, REPL, and LSP as the acceptance bar
- This slice should *not* yet:
  - migrate any other `io.*` / `fetch.*` / `build` / `config` / `store` entrypoint
  - accept implicit `String | Path` dual-typed arguments
  - add a public `Bytes` bridge
  - change path literal typing
  - introduce `Task<T>` / `Command` / `ProcessResult`
- The point of this slice is to prove that `Path` is no longer only a pure-domain value: it can enter one real canonical host path while the rest of stdlib migration remains explicitly staged.

### D-081: The next `PR-012` follow-up should lock invalid `io.readFilePath` diagnostics across consumers before any wider `io.*` migration

- After `io.readFilePath : Path -> String` exists, the next highest-value move is to prove that its failure path is surfaced identically by the canonical frontend, LSP, and REPL consumers.
- This slice should stay intentionally narrow:
  - add frontend, LSP, and REPL regressions for `io.readFilePath("...")`
  - assert the existing canonical `TypeMismatch` code and core message fragment
  - change implementation only if the new parity tests expose a consumer/rewrite drift
- This slice should *not* yet:
  - add new `io.*` typed-path bridges
  - redesign `std.io` error messages
  - widen effect/runtime-object scope beyond this single diagnostic parity check
- The point of this slice is to close the consumer-parity side of the first typed host bridge before extending `Path` across more host APIs.

### D-082: The next `PR-012` slice should add exactly one zero-argument typed-path host bridge, `io.currentDirPath`

- After `io.readFilePath` and its failure-path parity are locked, the next highest-value move is to avoid an unnecessary string round-trip for the current working directory.
- This slice should stay intentionally narrow:
  - add `io.currentDirPath() -> Path`
  - keep `io.currentDir() -> String` unchanged for compatibility
  - verify parity across stdlib runtime, canonical type checking, frontend, HIR evaluation, end-to-end, LSP, and REPL
- This slice should *not* yet:
  - add `homeDirPath`
  - add `pathExistsPath` / `isDirPath` / `isFilePath`
  - migrate any argument-taking `io.*` entrypoint beyond the already-added `io.readFilePath`
  - redesign effect boundaries or add `Bytes` / `Task<T>` / `Command`
- The point of this slice is to keep expanding typed `Path` across host boundaries one narrow adapter at a time while preserving explicit compatibility edges.

### D-083: The next `PR-012` slice should add exactly one predicate-style typed-path host bridge, `io.pathExistsPath`

- After `io.readFilePath` and `io.currentDirPath` are in place, the next highest-value move is to let typed `Path` participate in a minimal existence check without string round-tripping.
- This slice should stay intentionally narrow:
  - add `io.pathExistsPath(path: Path) -> Bool`
  - keep `io.pathExists(path: String) -> Bool` unchanged for compatibility
  - verify parity across stdlib runtime, canonical type checking, frontend, HIR evaluation, end-to-end, LSP, and REPL
- This slice should *not* yet:
  - add `io.isDirPath` / `io.isFilePath`
  - migrate any other typed `io.*` predicate as part of the same PR
  - redesign effect boundaries or widen the `Bytes` / `Task<T>` surface
- The point of this slice is to continue staged `Path` adoption with one simple, deterministic predicate bridge at a time.

### D-084: The next `PR-012` slice should add exactly one file-oriented typed-path predicate bridge, `io.isFilePath`

- After `io.pathExistsPath` is in place, the next highest-value move is to let typed `Path` distinguish file targets without falling back to string predicates.
- This slice should stay intentionally narrow:
  - add `io.isFilePath(path: Path) -> Bool`
  - keep `io.isFile(path: String) -> Bool` unchanged for compatibility
  - verify parity across stdlib runtime, canonical type checking, frontend, HIR evaluation, end-to-end, LSP, and REPL
- This slice should *not* yet:
  - add `io.isDirPath`
  - migrate any other typed `io.*` predicate as part of the same PR
  - redesign effect boundaries or widen the `Bytes` / `Task<T>` surface
- The point of this slice is to keep extending the typed file-path flow in small, deterministic steps without broadening the migration surface.

### D-085: The next `PR-012` slice should add exactly one directory-oriented typed-path predicate bridge, `io.isDirPath`

- After `io.pathExistsPath` and `io.isFilePath` are in place, the next highest-value move is to let typed `Path` distinguish directory targets without string round-tripping.
- This slice should stay intentionally narrow:
  - add `io.isDirPath(path: Path) -> Bool`
  - keep `io.isDir(path: String) -> Bool` unchanged for compatibility
  - verify parity across stdlib runtime, canonical type checking, frontend, HIR evaluation, end-to-end, LSP, and REPL
- This slice should *not* yet:
  - migrate `homeDir` to a typed path shape
  - add any other typed `io.*` predicate as part of the same PR
  - redesign effect boundaries or widen the `Bytes` / `Task<T>` surface
- The point of this slice is to complete the minimal deterministic path-predicate cluster one bridge at a time while still keeping the migration surface explicit and narrow.

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
- Landing sequence:
  1. extract current match coverage into an internal `pattern_analysis` module without expanding supported shapes
  2. replace direct `check_match_coverage` diagnostics emission with `PatternAnalysisResult -> diagnostics`
  3. add usefulness/subset-shadowing diagnostics inside the already-supported domain
  4. only then consider broader pattern classes or deeper solver integration
- Minimum first-slice internal API direction:
  - `analyze_match(scrutinee_ty, arms, ctx) -> PatternAnalysisResult`
  - helper types such as:
    - `PatternDomain`
    - `PatternCoverage`
    - `UnreachableArm { span, shadowed_by }`
  - the public/frontend surface does not change in this slice

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
- Prepared a refined `v1.2.0` release-notes body that emphasizes canonical pipeline progress, explicit AST compat boundaries, current platform scope, and the release validation gates.
- Started the next mainline step after the release cut by introducing an evaluator helper for caller-provided method-resolution tables and migrating command/test call sites toward that API.
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test eval --test end_to_end`
  - `cargo test -p neve run_ -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued the post-release highest-priority task by introducing a frontend-owned snippet analysis API for `neve eval`.
- Added `analyze_snippet_ast` plus `LoadedSnippetModule` in `crates/neve-frontend`, backed by `FrontendSession` in-memory module construction and loaded-module analysis.
- Migrated `neve eval` so:
  - ordinary snippets use the frontend-owned snippet analysis path
  - local imports no longer depend on temp-file/module-graph indirection
  - a narrow explicit compat branch remains only for unsupported std import shapes
- Added frontend integration coverage for:
  - snippet analysis with local imports against a rooted project directory
  - diagnostics attribution for loaded dependency modules during snippet analysis
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test -p neve eval_ -- --nocapture`
  - `cargo test --test frontend --test eval --test end_to_end`
  - `cargo clippy -p neve-frontend -p neve --all-targets -- -D warnings`
- Started the first `PR-010` slice by migrating `neve-config::module` off per-file AST evaluation without rewriting its record-driven import graph.
- Kept config-owned recursive `imports = ["./base.neve"]` loading in place, because this graph is data-driven rather than language-import-driven.
- Replaced the main file-evaluation path in `crates/neve-config/src/module.rs` with:
  - frontend analysis for source/path diagnostics
  - HIR evaluation with canonical method-resolution tables
  - a shared `module_from_value` translation step that keeps config semantics focused on evaluated records
- Added config integration coverage for:
  - loading a config module that uses language-level imports through frontend/HIR
  - surfacing frontend type diagnostics through `Module::load`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test -p neve-config`
  - `cargo test --test config`
  - `cargo clippy -p neve-config --all-targets -- -D warnings`
- Continued `PR-010` by migrating the root `neve-config::flake` evaluation path from AST compatibility execution to frontend/HIR.
- Replaced `Flake::load` / `Flake::parse` root evaluation with canonical frontend analysis plus HIR evaluation.
- Added explicit HIR closure invocation support for flake `outputs` by:
  - exposing a small public `Evaluator::call_value` helper
  - rebuilding the current flake's `outputs` value through frontend/HIR before calling it
- Kept flake input resolution, lock handling, archive/git/path materialization, and recursive input-flake traversal unchanged in this slice.
- Recorded a transitional flake-specific compromise: root flake execution now gates on parse/frontend-load diagnostics, not all type diagnostics, because current flake input records are still more dynamic than the type system can model.
- Added/updated flake regression coverage for:
  - recursive path-input output evaluation on the HIR path
  - `follows` alias output resolution on the HIR path
  - loading a root flake that uses language-level imports through frontend/HIR
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test -p neve-config`
  - `cargo test --test config`
  - `cargo clippy -p neve-config -p neve-eval --all-targets -- -D warnings`
- Completed the remaining `PR-010` product-path migration by removing AST-only parsing/evaluation from `neve build`.
- Split `neve build` source evaluation into two canonical branches:
  - `flake.neve` now goes through `neve-config::Flake`
  - plain `.neve` package files now go through frontend/HIR analysis plus HIR evaluation
- Kept derivation extraction, cache configuration, substitution, and builder execution local to `build.rs`, but removed its command-local AST evaluator ownership.
- Tightened flake package selection in `build.rs` so explicit package names are resolved through `Flake::get_package` instead of pretending they are top-level attributes on the raw root record.
- Added build-command regression coverage for:
  - plain package files with language-level imports on the frontend/HIR path
  - flake files with language-level imports on the canonical flake path
  - explicit flake package selection through the canonical flake API
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test -p neve build::tests -- --nocapture`
  - `cargo clippy -p neve -p neve-config -p neve-eval --all-targets -- -D warnings`
- Hardened the highest-priority record typing gap for canonical product paths by teaching typeck field access to continue through unknown/dynamic record candidates.
- Updated `crates/neve-typeck/src/check.rs` so:
  - `record.field` on known records still validates field presence strictly
  - `record.field` on `Var` / `Unknown` returns a fresh type instead of producing a spurious `non-record` error
  - `record?.field` now has real typing for known records, `Option[Record]`, and unknown/dynamic bases
- Removed the temporary flake parse-only diagnostic gate and restored full frontend/type-error gating for `neve-config::flake`.
- Added type-system regression coverage for:
  - dynamic record field chains on unknown function parameters
  - safe-field + coalesce on unknown parameters
  - preserving missing-field errors on known records
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck`
  - `cargo test -p neve-config`
  - `cargo test --test config`
  - `cargo test -p neve build::tests -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-config -p neve --all-targets -- -D warnings`
- Completed the next bounded record-hardening slice by introducing inference-only `TyKind::DynamicRecord` support across unification, type checking, and type formatting.
- Updated `crates/neve-typeck/src/check.rs` so repeated field access on the same unknown base accumulates field constraints instead of discarding them as unrelated fresh vars.
- Nested unknown-base field chains such as `inputs.dep.packages.default` now build nested `DynamicRecord` requirements that are checked again when the function is applied to a concrete record.
- Updated type formatting across frontend, REPL, typeck diagnostics, trait/type keys, and evaluator type keys so this internal type renders consistently as an open record shape.
- Added regression coverage for:
  - accumulating multiple nested field constraints on the same unknown base
  - rejecting concrete call sites that omit required nested fields after dynamic-record inference
  - preserving REPL/build/config product-path compilation after the new internal type variant
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck`
  - `cargo test -p neve-config`
  - `cargo test --test config`
  - `cargo test -p neve build::tests -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-config -p neve --all-targets -- -D warnings`
- Continued the record/display convergence slice by exposing a frontend helper for formatting a type against an explicit visible-name map.
- Reworked REPL semantic type rendering to delegate ordinary language-type formatting to frontend instead of maintaining another full copy of the formatter.
- Kept REPL-local formatting only for REPL pseudo-types that do not exist in ordinary frontend semantic output.
- Added regression coverage for:
  - LSP definition hovers showing nested dynamic-record shapes as open record types
  - REPL `:type` showing the same nested dynamic-record shape readably
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test lsp`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Tightened the remaining high-value safe-field gap by introducing an internal `SafeRecordBase` constraint for unknown `?.field` bases.
- Updated `crates/neve-typeck/src/check.rs` so unknown-base safe-field access:
  - no longer silently accepts concrete non-record callsites
  - still allows missing fields on records, because `?.field` may evaluate to `None`
  - accepts builtin `Option[Record]` callsites and checks present-field type consistency
- Kept this as an internal typechecker artifact only; no surface syntax or new language-level type form was added.
- Added regression coverage for:
  - rejecting `readName(42)` when `readName = fn(config) config?.name ?? "default"`
  - allowing missing-field record callsites on the same function
  - allowing builtin `Option[Record]` callsites
  - rejecting present-but-wrong field types
  - frontend/runtime end-to-end coverage for safe-field on `option.some(record)` and for frontend diagnostics on bad callsites
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo test -p neve-config`
  - `cargo test --test config`
  - `cargo clippy -p neve-typeck -p neve-frontend -p neve-lsp -p neve-config -p neve --all-targets -- -D warnings`
- Completed the first bounded optional-flow convergence slice by introducing a shared optional-flow normalization helper in `crates/neve-typeck/src/check.rs`.
- `try_result_type`, `coalesce_result_type`, and the option-bearing branch of `safe_field_result_type` now use the same internal recognition path for:
  - builtin `Option`
  - builtin `Result` where permitted
  - supported enum-like `Some/None` and `Ok/Err` cases where permitted
  - `Var` / `Unknown` as a shared “unknown flow base” outcome
- This slice also tightened known-invalid uses:
  - `41?` is now a direct type error
  - `41 ?? 0` is now a direct type error
- The slice intentionally stopped short of introducing another unknown-base constraint type for `?` or `??`; unknown-base precision beyond this remains a later step.
- Added regression coverage for:
  - rejecting known non-optional `?`
  - rejecting known non-optional `??`
  - frontend diagnostics for the same invalid uses
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Closed the next optional-flow loop by aligning evaluator behavior with the stricter canonical type checker for known-invalid `?` / `??` uses.
- Updated both HIR evaluation and AST compat so they now reject:
  - non-optional scalar values under `?`
  - non-optional scalar values under `??`
  - unrelated enum/variant values that previously flowed through unchanged
- Kept the slice intentionally narrow:
  - no new unknown-base constraint type for `?` / `??`
  - no change to successful builtin `Option`, builtin `Result`, or supported enum-like `Some/None` and `Ok/Err` behavior
- Added direct evaluator regression coverage for:
  - HIR `41?` rejection
  - HIR `41 ?? 0` rejection
  - AST compat `41?` rejection
  - AST compat `41 ?? 0` rejection
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-eval -p neve --all-targets -- -D warnings`
- Started the first `PR-015` slice by extracting match coverage / reachability logic into `crates/neve-typeck/src/pattern_analysis.rs`.
- `TypeChecker` now uses `PatternAnalysisResult -> diagnostics` instead of embedding the full match-coverage state machine directly in `check.rs`.
- This slice intentionally kept the supported analysis domain unchanged:
  - `Bool`
  - `Unit`
  - builtin `Option`
  - builtin `Result`
  - user enums
- Added explicit regression coverage for the conservative first-slice guard policy:
  - guarded arms do not make a match exhaustive
  - guarded arms do not make later unguarded arms unreachable
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck --all-targets -- -D warnings`
- Continued `PR-015` with the first bounded usefulness/subset-shadowing slice inside the new `pattern_analysis` layer.
- Added an explicit distinction between:
  - coverage space used for exhaustiveness
  - possible match space used for usefulness / unreachable-pattern checks
- This now catches the first high-value subset-shadowing cases inside the already-supported domains:
  - builtin `Option`: `Some(_)` before later `Some(value)`
  - builtin `Result`: `Ok(_)` before later `Ok(value)`
  - user enums: `Variant(_)` before later `Variant(value)`
  - bool: `true | false` before later bool arms
- Kept the slice intentionally conservative:
  - guarded arms still do not contribute to exhaustiveness
  - guarded arms still do not make later arms unreachable
  - no broader pattern-domain expansion or solver rewrite
- Added regression coverage in:
  - `tests/typeck.rs` for builtin option/result, user-enum, bool-or-pattern, and guard policy
  - `tests/end_to_end.rs` for frontend-visible unreachable-pattern warnings on the canonical path
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve --all-targets -- -D warnings`
- Continued `PR-015` by making usefulness provenance first-class inside `PatternAnalysisResult`.
- The extracted analysis layer now records per-arm usefulness state instead of only emitting a final flat unreachable-arm list:
  - `Useful`
  - `Redundant { witness_span, reason }`
  - `GuardedIgnored`
  - `NotAnalyzed`
- Unreachable-pattern diagnostics are now derived from this per-arm analysis result instead of being reconstructed from incidental control flow.
- This improved witness precision for the already-supported domains:
  - subset-shadowing warnings can now point back to the earlier arm that actually witnesses the redundancy
  - single-variant enum irrefutable constructor coverage is now locked in with regression coverage
- Added regression coverage in `tests/typeck.rs` for:
  - previous-label witness precision on builtin `Option` subset-shadowing
  - single-variant enum irrefutable constructor coverage making later arms unreachable
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve --all-targets -- -D warnings`
- Continued `PR-015` by consuming `RedundancyReason` in unreachable-pattern diagnostics instead of leaving it as unused internal metadata.
- Unreachable warnings now distinguish between:
  - full coverage by previous arms
  - subset-shadowing by an earlier arm that already covers the same case space
- The user-facing message remains `unreachable pattern`, but the previous-arm label and note are now semantically different for the two cases.
- Added targeted regression coverage in `tests/typeck.rs` for:
  - full-coverage wildcard warnings using the “matches all remaining values” wording
  - subset-shadowing warnings using the “already covers this case” wording
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve --all-targets -- -D warnings`
- Continued `PR-015` by stabilizing enum-domain pattern diagnostics around declaration order instead of hash-map iteration order.
- `EnumInfo` now preserves variant declaration order explicitly, and pattern-analysis formatting/iteration for user-enum missing patterns now follows that order.
- This makes user-visible output deterministic for the already-supported enum domain:
  - `missing patterns: ...` notes are now stable
  - canonical frontend diagnostics no longer depend on `HashMap` iteration when formatting missing enum variants
- Added regression coverage for:
  - typeck-level user-enum missing-pattern note ordering
  - frontend canonical-path user-enum missing-pattern note ordering
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve --all-targets -- -D warnings`
- Started the first `PR-016` slice by canonicalizing trait-impl method signatures inside `TraitResolver` during the pre-body impl-signature pass.
- The change is intentionally narrow:
  - trait impl associated-type bindings stored in the resolver are normalized before body checking proceeds
  - trait impl method signatures stored in the resolver are rewritten to their concrete `Self` / associated-type forms before method-call inference reads them
  - the slice does not yet broaden to inherent impls or add a new standalone associated-type projection table
- Added focused regressions for canonical assoc-return method calls across:
  - `tests/typeck.rs`
  - `tests/lsp.rs`
  - `neve-cli/src/commands/repl.rs`
  - `tests/end_to_end.rs`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-016` by exposing explicit associated-type projection resolutions through the canonical semantic artifact instead of leaving them implicit inside type checking.
- The implementation remains narrow and mainline-aligned:
  - `TypeChecker` now records resolved `Self.Item` projections keyed by explicit type-use span
  - `ModuleSemantics` now carries that projection table alongside method resolutions and normalized type tables
  - `FrontendSession` clears the projection table between loaded modules just like per-module method resolutions
- Added focused regression coverage for artifact exposure in `tests/frontend.rs`, while keeping existing LSP / REPL / canonical-path behavior green.
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-016` with the first consumer-adoption slice for explicit associated-type projections.
- The change remains narrow and mainline-aligned:
  - `neve-frontend` now exposes a projection-aware type-use formatter for explicit type spans
  - `neve-lsp` now collects semantic hovers for method-signature type annotations and reads canonical projection results when available
  - explicit `Self.Item` spans inside impl method parameter/return annotations now hover as their concrete projected type, while trait-definition `Self.Item` still shows its source-level shape
- Added focused regressions in `tests/lsp.rs` for:
  - impl method parameter `Self.Item` hover resolving to the concrete projected type
  - impl method return `Self.Item` hover resolving to the concrete projected type
  - trait method `Self.Item` hover preserving the source-level `Self.Item` display when no concrete projection exists
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo clippy -p neve-frontend -p neve-lsp --all-targets -- -D warnings`
- Continued `PR-016` by making impl-signature mismatch diagnostics consume canonical associated-type projection artifacts.
- The change is intentionally narrow:
  - the existing `impl method ... does not match trait ... signature` diagnostic now attaches projection-aware labels for explicit `Self.Item` type-use spans
  - those labels are sourced from the canonical projection table already recorded during type checking
  - the slice does not broaden to other trait diagnostics yet
- Added focused regressions for:
  - typeck-level impl-signature mismatch diagnostics carrying projection-aware labels
  - frontend canonical diagnostics preserving those projection-aware labels after name rewriting
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend --all-targets -- -D warnings`
- Continued `PR-016` by extending projection-aware diagnostics from impl-signature mismatch reporting to impl-body return mismatches.
- The change remains narrow and on the same canonical path:
  - `impl method ... return type` diagnostics now attach projection-aware labels for explicit `Self.Item` spans
  - those labels are sourced from the same canonical projection table already used by the previous impl-signature mismatch slice
  - the slice still does not broaden to missing methods, associated-type bounds, or dispatch ambiguity
- Added focused regressions for:
  - typeck-level impl-body return mismatch diagnostics carrying projection-aware labels
  - frontend canonical diagnostics preserving those projection-aware labels after rewrite
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend --all-targets -- -D warnings`
- Continued `PR-016` by correcting the canonical projection boundary before any broader consumer expansion.
- The change remains deliberately narrow:
  - shared `assoc_projection_resolutions` no longer record impl-specific concrete types for trait declaration `Self.Item` spans during impl-signature checking
  - impl-side signature spans and concrete use-site spans still record canonical projections
  - impl-signature mismatch diagnostics keep their projection-aware labels via a local diagnostic-only projection map rather than polluting the shared semantic artifact
- Added focused regressions for:
  - frontend semantic artifacts keeping trait `Self.Item` spans source-level while still recording impl spans in the same module
  - LSP hover preserving trait `Self.Item` source shape even when a concrete impl is present in the same module
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend -p neve-lsp --all-targets -- -D warnings`
- Continued `PR-016` by canonicalizing impl-side assoc-type bound diagnostics without reordering the broader impl-check pipeline.
- The change remains narrow and mainline-aligned:
  - `check_assoc_type_bounds` now resolves impl/default associated-type bindings through a local canonical helper before checking trait bounds
  - the helper recursively normalizes `Self.OtherAssoc` chains, substitutes `Self`, and carries a narrow cycle guard so bad assoc definitions fail locally instead of looping
  - assoc-bound diagnostics now render the canonical resolved type in labels/notes rather than the raw impl-side type syntax
  - the slice does not broaden to method-call failure semantics or wider trait-solver work
- Added focused regressions for:
  - impl assoc bounds succeeding through canonical `Self.Alias -> Int` resolution
  - assoc-bound failures rendering canonical `Int` instead of raw `Self.Alias`
  - cyclic associated-type definitions producing a local type diagnostic instead of recursing indefinitely
  - frontend canonical diagnostics preserving the same canonical assoc-bound rendering
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend --all-targets -- -D warnings`
- Continued `PR-016` by collapsing the older shallow impl-signature assoc path onto the proven canonical helper.
- The change remains narrow and mainline-aligned:
  - `check_all_impls` now computes one canonical impl-assoc map per impl and reuses it for assoc-bound checks, impl-signature checking, and `TraitResolver` normalization
  - trait default associated types and `Self.OtherAssoc` alias chains now flow through the same canonical resolver path as explicit impl-side assoc bindings
  - this upgrades canonical method-return / tooling-visible results without touching broader method-call failure semantics
- Added focused regressions for:
  - method calls whose return type depends on a default associated type alias chain now type-check as the concrete projected type
  - LSP definition hover for the resulting binding now shows the same concrete type from the canonical path
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend -p neve-lsp --all-targets -- -D warnings`
- Continued `PR-016` by codifying method-call dispatch precedence in tests/docs before any breaking failure-semantics change.
- The change remains intentionally narrow:
  - type checking and HIR evaluation now both carry explicit comments documenting the canonical dispatch order:
    1. receiver method dispatch
    2. callable-target fallback
  - frontend tests now lock that only the dispatch branch records `method_resolutions`, while the callable-target fallback branch leaves that semantic table empty
  - typeck/LSP/end-to-end tests now distinguish the dispatch-precedence branch from the fallback branch using same-name callable examples
  - unresolved method calls with no callable fallback are currently test-locked as the existing lowered-target failure path rather than being silently rewritten into a new missing-method diagnostic
- Documentation now reflects the same model in:
  - `docs/reference/spec.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-frontend -p neve-lsp -p neve-eval --all-targets -- -D warnings`
- Continued the method-call semantics line by replacing the temporary lowered-target `undefined global` failure with a dedicated call-site missing-method diagnostic for the narrow no-method / no-callable branch.
- The change remains narrow and mainline-aligned:
  - dispatch precedence is unchanged: receiver method dispatch first, callable-target fallback second
  - only the branch where the lowered callable target is unresolved now emits a dedicated type diagnostic at the original method-call span
  - callable-target fallback success and callable-target-specific failures continue to use the ordinary fallback path
  - the new diagnostic is carried by a stable `UnknownMethod` error code and explains both the missing receiver method and the absence of a callable fallback
- Added focused regressions for:
  - typeck reporting the dedicated missing-method diagnostic instead of generic `undefined global`
  - frontend preserving the same diagnostic code/message on the canonical path
  - LSP surfacing the same dedicated diagnostic from `Document`
  - end-to-end frontend analysis locking the dedicated diagnostic on the real pipeline
- Documentation now reflects the dedicated failure branch in:
  - `docs/reference/spec.md`
  - `docs/reference/diagnostics.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Continued `PR-017` by locking the optional-flow tooling matrix for valid imported source shapes rather than broadening implicit tool scope.
- The change remains intentionally narrow:
  - added direct LSP hover regressions for imported `option.some(41)? + 1`, `option.none ?? 5`, and `option.some(record)?.field ?? default`
  - added matching REPL `:type` regressions using the same explicit `import std.option as option` setup
  - did not broaden REPL or LSP default scope for builtin namespaces; the tests now exercise the canonical imported source shape instead
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-017` by locking invalid optional-flow diagnostics across frontend, REPL, and LSP before any further inference work.
- The change remained test-first and consumer-focused:
  - added frontend regressions for `41?`, `41 ?? 0`, and `42?.name ?? "default"` that assert the canonical `TypeMismatch` code plus the existing core message
  - added matching LSP document-diagnostics regressions for the same invalid shapes
  - added matching REPL `:type` diagnostic regressions by asserting `TypeQueryError::Diagnostics` carries the same canonical code/message fragments
  - no implementation change was needed, which confirms the current frontend, REPL, and LSP paths already surface the same canonical optional-flow failures
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Started the first implementation slice of `PR-011` by introducing dormant runtime-object identity for `Path` and `Bytes` without changing stdlib contracts.
- The change remains intentionally narrow and mainline-aligned:
  - added builtin named-type identities plus constructors for the planned runtime-object family in `crates/neve-typeck/src/builtin_types.rs`
  - re-exported the new builtin runtime-object types from `neve-typeck` so formatting and future consumers can reference a stable canonical identity
  - introduced internal `Value::Path` and `Value::Bytes` runtime values in `crates/neve-eval/src/value.rs`
  - shifted REPL-only pseudo-type IDs away from the builtin reserved range so the new runtime-object identities do not collide with test/runtime formatting helpers
  - made `typeOf`, runtime-value formatting, stable value keys, and REPL runtime type formatting aware of `Path` / `Bytes`
  - kept the slice dormant on purpose: no stdlib contract changes, no effect APIs, and no path-literal typing change yet
- Added focused regressions for:
  - `typeOf` reporting `Path` for internal path runtime objects
  - `typeOf` reporting `Bytes` for internal bytes runtime objects
  - REPL runtime-value typing/formatting recognizing `Path` and `Bytes` as builtin named types instead of leaking pseudo-type collisions
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test -p neve-eval builtin::tests -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-typeck -p neve-eval -p neve --all-targets -- -D warnings`
- Continued `PR-011` by exposing the first minimal user-visible `Path` bridge while keeping the old string-heavy stdlib contracts intact.
- The change remained intentionally narrow and mainline-aligned:
  - added `path.fromString : String -> Path` to `std.path` as the first canonical constructor bridge into the new runtime-object family
  - kept `path.join` / `path.parent` / `path.filename` / `path.extension` / `path.is_absolute` on their existing string-based contracts for now
  - relied on the already-canonical `toString` / `typeOf` surface to display and introspect `Path` values instead of introducing a larger `std.path` API all at once
  - kept `Bytes` user-internal for now, since there is still no deliberately designed stdlib module contract for it
- Added focused regressions for:
  - `std.path` directly returning a `Value::Path` runtime object from `path.fromString`
  - canonical type checking accepting `toString(path.fromString(...))` while rejecting accidental use of `Path` with the still-legacy string-only `path.is_absolute`
  - frontend, HIR evaluation, end-to-end parity, and REPL `:type` all recognizing `path.fromString(...)` as producing `Path`
- Documentation now reflects the bridge and its still-limited scope in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve --all-targets -- -D warnings`
- Started the first `PR-012` slice by adding only the minimal typed-path compatibility adapters planned in `D-078`.
- The change remained intentionally narrow and mainline-aligned:
  - added `path.joinPath : Path -> String -> Path`
  - added `path.parentPath : Path -> Option[Path]`
  - added `path.isAbsolutePath : Path -> Bool`
  - kept all existing string-based `std.path` functions unchanged
  - did not migrate `io.*`, `fetch.*`, or path literals to `Path`
  - did not expose a broader `Bytes` stdlib surface yet
- Added focused regressions for:
  - stdlib direct builtin behavior returning/accepting `Path` runtime values
  - canonical type checking of typed-path composition while still rejecting accidental use of string receivers in the new adapter family
  - frontend acceptance of typed-path adapter flows
  - HIR evaluation and end-to-end parity for typed-path composition
  - REPL `:type` and LSP hover showing `Path` for typed-path adapter results
- Documentation now reflects the widened but still partial `Path` story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-011` with the next dormant runtime-object skeleton slice for command/process identity.
- The change remained intentionally narrow and mainline-aligned:
  - added opaque `Value::Command` and `Value::ProcessResult` runtime objects in `crates/neve-eval/src/value.rs`
  - taught `typeOf`, runtime formatting, stable-key generation, equality, and AST-compat runtime-type keys to recognize those objects as first-class runtime identities instead of collapsing them into `Record`
  - kept the new object constructors test-only and internal, so this slice does not expose a new public command/process API surface yet
  - left `io.exec`, `io.execShell`, and `io.execWith` on their existing `String`/`Record` contracts, so command execution remains explicitly outside this slice
- Added focused regression coverage for:
  - `typeOf` reporting `Command` and `ProcessResult`
  - opaque formatting that stays distinct from record rendering
  - stable-key and equality behavior that keeps the new objects distinct from record-shaped lookalikes
- Validation completed with:
  - `cargo test -p neve-eval --lib`
  - `cargo clippy -p neve-eval --all-targets -- -D warnings`
- Continued `PR-011` by exposing the first minimal user-visible `Command` bridge while keeping process execution contracts unchanged.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.command : String -> List[String] -> Command` to `std.io` as the first canonical constructor bridge into the command runtime-object family
  - kept `io.exec`, `io.execShell`, and `io.execWith` on their existing `String` / `Record` contracts, so this slice does not migrate execution onto `Command` yet
  - kept `ProcessResult` internal-only as a runtime identity, since there is still no deliberately designed public process-result API surface
  - aligned frontend, REPL, and LSP type presentation so `Command` appears as a builtin named type instead of leaking as `Record` or staying hidden
- Added focused regressions for:
  - `std.io` directly returning a `Value::Command` runtime object from `io.command`
  - canonical type checking accepting `io.command("printf", ["neve"])` while rejecting accidental use of `Command` with the still-legacy string-only `io.execShell`
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` all recognizing `io.command(...)` as producing `Command`
- Documentation now reflects the widened but still staged command story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-011` by exposing the first minimal `Command`-consuming execution bridge while keeping the legacy exec wrappers intact.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.execCommand : Command -> ProcessResult` to `std.io` as the first canonical execution bridge over the new command runtime-object family
  - kept `io.exec`, `io.execShell`, and `io.execWith` on their existing `String` / `Record` contracts, so this slice does not migrate the legacy execution surface
  - made `ProcessResult` publicly reachable only as the return type of `io.execCommand`, without adding a broader inspector/method API in the same slice
  - aligned frontend, REPL, and LSP type presentation so `ProcessResult` appears as a builtin named type rather than leaking as `Record`
- Added focused regressions for:
  - `std.io` directly returning a `Value::ProcessResult` runtime object from `io.execCommand(io.command(...))`
  - canonical type checking accepting `io.execCommand(io.command(...))` while still rejecting accidental `String` input to `io.execCommand`
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` all recognizing `io.execCommand(...)` as producing `ProcessResult`
- Documentation now reflects the widened but still staged command/process story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-011` with the first minimal pure `ProcessResult` inspector bridge.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.processSuccess : ProcessResult -> Bool` to `std.io` as the first dedicated inspector over the new process-result runtime-object family
  - kept `ProcessResult` otherwise opaque, so this slice does not add `code`, `stdout`, or `stderr` accessors yet
  - kept `io.exec`, `io.execShell`, and `io.execWith` on their existing `String` / `Record` contracts, so the legacy execution surface still remains in place
  - aligned frontend, REPL, and LSP type presentation so the new inspector result is shown canonically as `Bool`
- Added focused regressions for:
  - `std.io` directly consuming a `Value::ProcessResult` runtime object via `io.processSuccess`
  - canonical type checking accepting `io.processSuccess(io.execCommand(io.command(...)))` while still rejecting accidental non-`ProcessResult` input
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` all recognizing `io.processSuccess(...)` as producing `Bool`
- Documentation now reflects the widened but still staged process-result story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-011` with the first textual `ProcessResult` data accessor bridge.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.processStdout : ProcessResult -> String` to `std.io` as the first textual accessor over the new process-result runtime-object family
  - kept `ProcessResult` otherwise staged, so this slice still does not add `code` or `stderr` accessors
  - kept `io.exec`, `io.execShell`, and `io.execWith` on their existing `String` / `Record` contracts, so the legacy execution surface still remains in place
  - aligned frontend, REPL, and LSP type presentation so the new accessor result is shown canonically as `String`
- Added focused regressions for:
  - `std.io` directly consuming a `Value::ProcessResult` runtime object via `io.processStdout`
  - canonical type checking accepting `io.processStdout(io.execCommand(io.command(...)))` while still rejecting accidental non-`ProcessResult` input
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` all recognizing `io.processStdout(...)` as producing `String`
- Documentation now reflects the widened but still staged process-result story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-011` with the first numeric `ProcessResult` data accessor bridge.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.processCode : ProcessResult -> Int` to `std.io` as the first numeric accessor over the new process-result runtime-object family
  - kept `ProcessResult` otherwise staged, so this slice still does not add `stderr` accessors or any broader record-style projection API
  - kept `io.exec`, `io.execShell`, and `io.execWith` on their existing `String` / `Record` contracts, so the legacy execution surface still remains in place
  - aligned frontend, REPL, and LSP type presentation so the new accessor result is shown canonically as `Int`
- Added focused regressions for:
  - `std.io` directly consuming a `Value::ProcessResult` runtime object via `io.processCode`
  - canonical type checking accepting `io.processCode(io.execCommand(io.command(...)))` while still rejecting accidental non-`ProcessResult` input
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` all recognizing `io.processCode(...)` as producing `Int`
- Documentation now reflects the widened but still staged process-result story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-011` with the first stderr-oriented `ProcessResult` data accessor bridge.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.processStderr : ProcessResult -> String` to `std.io` as the first stderr accessor over the new process-result runtime-object family
  - kept `ProcessResult` on explicit accessor-style exposure, so this slice still does not turn process results into a general public record projection API
  - kept `io.exec`, `io.execShell`, and `io.execWith` on their existing `String` / `Record` contracts, so the legacy execution surface still remains in place
  - aligned frontend, REPL, and LSP type presentation so the new accessor result is shown canonically as `String`
- Added focused regressions for:
  - `std.io` directly consuming a `Value::ProcessResult` runtime object via `io.processStderr`
  - canonical type checking accepting `io.processStderr(io.execCommand(io.command(...)))` while still rejecting accidental non-`ProcessResult` input
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` all recognizing `io.processStderr(...)` as producing `String`
- Documentation now reflects the widened but still staged process-result story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-011` by converging the legacy `io.exec` implementation onto the canonical command/process runtime path without changing its public record contract.
- The change remained intentionally narrow and mainline-aligned:
  - kept `io.exec : String -> List<String> -> #{ code, success, stdout, stderr }` unchanged as a user-visible API
  - reimplemented `io.exec` internally as a projection from the canonical `Command -> ProcessResult` execution path, instead of maintaining a fully separate process-execution branch
  - preserved the legacy `io.exec:` error prefix and record result shape, so this slice does not migrate callers to `Command` or `ProcessResult`
  - left `io.execShell` and `io.execWith` on their existing legacy string/record implementations
- Added focused regressions for:
  - direct stdlib agreement between `io.exec(...)` and the canonical `io.execCommand(io.command(...))` plus `ProcessResult` accessors
  - HIR evaluation and end-to-end parity confirming the legacy record surface still matches the canonical command/process projection
  - compatibility of the legacy `io.exec:` error prefix for missing-program failures
- Documentation now reflects the narrower but more canonical process-execution story in:
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-012` with the directory-oriented typed-path predicate bridge planned in `D-085`.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.isDirPath : Path -> Bool`
  - kept the existing `io.isDir : String -> Bool` contract unchanged for compatibility
  - did not migrate `homeDir` or add any other typed predicate in the same slice
- Added focused regressions for:
  - stdlib direct builtin behavior accepting `Value::Path` and rejecting accidental `String` input to `io.isDirPath`
  - canonical type checking accepting `io.isDirPath(path.fromString(...))` while still rejecting `io.isDirPath("...")`
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` for the new directory predicate bridge
- Documentation now reflects the widened but still staged `std.io` typed-path story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-012` with the file-oriented typed-path predicate bridge planned in `D-084`.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.isFilePath : Path -> Bool`
  - kept the existing `io.isFile : String -> Bool` contract unchanged for compatibility
  - did not add `io.isDirPath` or any other typed predicate in the same slice
- Added focused regressions for:
  - stdlib direct builtin behavior accepting `Value::Path` and rejecting accidental `String` input to `io.isFilePath`
  - canonical type checking accepting `io.isFilePath(path.fromString(...))` while still rejecting `io.isFilePath("...")`
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` for the new file predicate bridge
- Documentation now reflects the widened but still staged `std.io` typed-path story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-012` with the predicate-style typed-path host bridge planned in `D-083`.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.pathExistsPath : Path -> Bool`
  - kept the existing `io.pathExists : String -> Bool` contract unchanged for compatibility
  - did not add `io.isDirPath` or `io.isFilePath`
- Added focused regressions for:
  - stdlib direct builtin behavior accepting `Value::Path` and rejecting accidental `String` input to `io.pathExistsPath`
  - canonical type checking accepting `io.pathExistsPath(path.fromString(...))` while still rejecting `io.pathExistsPath("...")`
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` for the new predicate bridge
- Documentation now reflects the widened but still staged `std.io` typed-path story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-012` with the narrow consumer-parity follow-up planned in `D-081`.
- The change stayed intentionally test-first and mainline-aligned:
  - added frontend, LSP, and REPL regressions for the invalid shape `io.readFilePath("...")`
  - asserted the existing canonical `TypeMismatch` code plus the core `type mismatch` message fragment
  - no implementation change was needed, which confirms the current frontend, LSP, and REPL paths already surface the same canonical failure for the new typed-path host bridge
- Validation completed with:
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
- Continued `PR-012` with the zero-argument typed-path host bridge planned in `D-082`.
- The change remained intentionally narrow and mainline-aligned:
  - added `io.currentDirPath() -> Path` as the first `std.io` entrypoint that produces a typed path runtime object directly from the host
  - kept the existing `io.currentDir() -> String` contract unchanged for compatibility
  - did not add `homeDirPath` or any new argument-taking typed `io.*` entrypoint
- Added focused regressions for:
  - stdlib direct builtin behavior returning `Value::Path` from `io.currentDirPath`
  - canonical type checking accepting `toString(io.currentDirPath())` while keeping `io.currentDir()` unchanged
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` for the new zero-argument typed-path bridge
- Documentation now reflects the widened but still staged `std.io` typed-path story in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
- Continued `PR-012` with the first typed host-boundary bridge planned in `D-080`.
- The change remains intentionally narrow and mainline-aligned:
  - added `io.readFilePath : Path -> String` as the first `std.io` entrypoint that consumes a typed path runtime object
  - kept the existing `io.readFile : String -> String` contract unchanged for compatibility
  - did not migrate any other `io.*`, `fetch.*`, or platform entrypoint to `Path`
  - did not add implicit `String | Path` dual-typed parameters or widen the `Bytes` surface
- Added focused regressions for:
  - stdlib direct builtin behavior accepting `Value::Path` and rejecting accidental `String` input to `io.readFilePath`
  - canonical type checking accepting `io.readFilePath(path.fromString(...))` while still rejecting `io.readFilePath("...")`
  - frontend acceptance, HIR evaluation, end-to-end parity, LSP hover, and REPL `:type` for the new typed-path host bridge
- Documentation now reflects the first `Path`-consuming `std.io` bridge in:
  - `docs/reference/api.md`
  - `docs/project/feature-matrix.md`
  - `docs/project/language-roadmap.md`
- Validation completed with:
  - `cargo fmt --all`
  - `cargo test --test std -- --nocapture`
  - `cargo test --test typeck -- --nocapture`
  - `cargo test --test frontend -- --nocapture`
  - `cargo test --test eval -- --nocapture`
  - `cargo test --test end_to_end -- --nocapture`
  - `cargo test --test lsp -- --nocapture`
  - `cargo test -p neve repl_ -- --nocapture`
  - `cargo clippy -p neve-std -p neve-typeck -p neve-eval -p neve-frontend -p neve-lsp -p neve --all-targets -- -D warnings`
