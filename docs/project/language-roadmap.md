<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Language Completion Roadmap</h1>

<p><em>语言完备化与系统脚本化路线图</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong> ·
  <strong><a href="roadmap.md">Project Roadmap</a></strong>
</p>

</div>

---

这份文档只回答一件事：

**接下来怎样把 Neve 从“语法很多的原型”做成“真正独立可用的语言”，以及怎样把它做成能替代大部分 shell 工作的系统级语言。**

这不是宣传文档，也不是泛泛路线图。
它是工程执行文档，所以会直接回答：

- 现在到底缺什么
- 先做什么，后做什么
- 哪些事情现在不能碰
- 做到什么程度才算“真的完成”

这份文档比总路线图更严格，核心原则只有一句：

**在现有语法还没有形成单一、闭环、可验证的语义管线之前，不再继续优先扩张表面语法。**

## 一页读懂 / Quick Read

如果你只想先看最重要的内容，看这一节就够了。

### 1. 当前最真实的问题

Neve 现在最大的问题，不是“语法还不够多”，而是“同一门语言在不同路径上的语义还不一致”。

具体来说：

- parser 支持的东西，比 lowering 和 typeck 真正稳住的东西多
- AST evaluator 和 HIR evaluator 不是同一套语义
- 端到端测试基线已经换成真实执行路径，但覆盖深度仍小于语言表面
- 文档里有些 `Complete` 声明高于真实状态
- 系统能力已经开始加了，但 effect boundary 还没有正式定下来

### 2. 这份路线图的核心判断

- 第一，不要继续优先加语法，要先做语义收敛
- 第二，不要把“加几个 `exec` builtin”误当成“能替代 Bash”
- 第三，Neve 不需要兼容 Bash 语法，但必须覆盖 Bash 的主要能力
- 第四，Neve 不需要先做成“交互式 shell”，先做成“强类型系统脚本语言”更重要
- 第五，Neve 要先成为“自洽语言”，再成为“生态系统”

### 3. 真正的开工顺序

1. 先做现实校准
   把所有 feature 的真实支持情况列清楚，把假的完整度去掉。
2. 再做语义收敛
   统一 parser -> HIR -> typeck -> eval 这条链。
3. 再做编译器级类型语义
   补齐 exhaustiveness、unreachable pattern、trait method 语义。
4. 再做运行时与 effect boundary
   引入 `Path`、`Command`、`ProcessResult` 等真实运行时对象。
5. 再做 shell 替代能力
   管道、重定向、超时、取消、信号、shebang、脚本入口。
6. 最后做生态闭环
   lockfile、registry、包元数据、兼容性和发布策略。

### 4. 这份文档最重要的结论

在 Phase 0 和 Phase 1 完成前，Neve 不应该继续把主要精力放在：

- 新语法
- 宏系统
- HKT
- FFI
- 全 POSIX 兼容
- 交互式 shell 替代

### 5. 你应该怎么用这份文档

- 如果你要判断大方向：看 `一页读懂`、`决策门`、`阶段计划`
- 如果你要安排开发顺序：看 `工作包目录`、`执行顺序`
- 如果你要直接开工：看 `当前立即优先级`、`下一执行批次`

## 怎么读 / How To Read

为了让中文读者更容易使用，这份文档按三层来组织：

### 第一层：决策层

回答“项目接下来到底该往哪走”：

- `一页读懂`
- `决策门`
- `阶段计划`

### 第二层：管理层

回答“这些工作之间有什么依赖关系”：

- `能力矩阵`
- `工作包目录`
- `执行顺序`
- `延后项`

### 第三层：执行层

回答“下一步到底改哪些文件、做哪些 PR”：

- `当前立即优先级`
- `下一执行批次`

## 这份文档的写法 / Why It Looks Like This

这份文档故意不用“很漂亮但没法执行”的路线图写法。
它要像施工图，而不是宣传册。

所以你会看到很多：

- 工作包编号
- 决策门
- 进入条件
- 验收条件
- 非目标

这是为了避免后面再次出现：

- 目标说得很大，但不知道第一刀砍哪里
- 做了很多事，但没有证明做到了
- 新增功能越来越多，但核心语义一直没收敛

## 术语对照 / Glossary

这份文档里有些术语如果不先约定，中文读者会读得很别扭。
下面统一解释一次。

| Term | 中文建议理解 | 在本文里的意思 |
|------|--------------|----------------|
| canonical pipeline | 规范执行管线 | 项目最终承认的唯一语义主链路 |
| semantic convergence | 语义收敛 | parser、lowering、typeck、eval 不再各说各话 |
| lowering | 降级 / 降到 HIR | 把 AST 变成 HIR 的那一步 |
| effect boundary | 副作用边界 | 纯表达式和 I/O/进程/系统修改的分界线 |
| shell replacement | shell 替代 | 不是兼容 Bash 语法，而是覆盖主要 shell 工作 |
| structured runtime | 结构化运行时 | 用 `Path`、`Command` 这类类型，而不是字符串乱传 |
| feature matrix | 能力矩阵 / 支持矩阵 | 逐项标注功能在哪些层真正支持 |
| validation corpus | 验证语料 | 用来证明路线图阶段真的完成的一组真实样本 |
| work package | 工作包 | 比阶段更小、可以直接开 issue 和 PR 的任务单元 |
| decision gate | 决策门 | 不先定下来就会导致后面返工的关键选择 |

## Targets / 目标

### Target A: Complete Programming Language / 目标 A：独立完备语言

Neve should have:

- one canonical execution pipeline
- a coherent static semantics model
- stable module, type, trait, and error semantics
- a real end-to-end test corpus
- tooling that reflects the actual language, not a subset or placeholder

### Target B: System-Level Language / 目标 B：系统级语言

Neve should support:

- structured filesystem and process operations
- explicit effects and predictable failure semantics
- automation, provisioning, and configuration tasks
- safe command composition without shell quoting hazards
- integration with build, package, and host configuration workflows

### Target C: Bash Replacement Layer / 目标 C：替代 Bash 的能力层

Neve does **not** need to copy Bash syntax.
It does need to cover the core capabilities people use Bash for:

- invoking commands
- wiring pipelines
- redirecting I/O
- controlling environments and working directories
- handling exit status and failures
- performing filesystem automation
- orchestrating long-running jobs and service entrypoints

## Current Reality / 当前现实

Neve already has a rich parser surface, but it is not yet a single closed language implementation.

当前代码库的真实状态更接近：

- parser surface is broad
- AST evaluation is more feature-complete than HIR evaluation
- type checking and lowering do not preserve every language construct faithfully
- system-facing effects already exist, but without a formal effect boundary
- several "complete" claims in docs are ahead of the actual integration state

The gap is therefore not mainly "missing syntax".
The gap is "missing semantic convergence".

现在的主缺口不是“语法还不够多”，而是“语义还没有收敛”。

## Guiding Principles / 指导原则

- Single truth: parser, lowering, type checker, evaluator, formatter, and LSP must agree on the same language.
- Core first: pure language semantics must be stable before expanding effectful APIs aggressively.
- Effects explicit: I/O, process, and host mutations must be isolated from pure evaluation.
- Structured over stringly: commands, paths, streams, and environment should become typed values, not ad hoc strings.
- Test what is real: placeholder end-to-end tests do not count as language completion.
- No syntax inflation: new syntax must earn its way in by proving semantic closure and toolchain support.

## Completion Dimensions / 完备性的维度

Neve becoming a "complete language" is not one thing.
It is the intersection of five dimensions:

Neve 要成为“完备语言”不是单点完成，而是五个维度同时闭环：

| Dimension | What it means | Current pressure |
|-----------|---------------|------------------|
| Surface syntax | The spec can express the intended language forms | Already broad |
| Semantic fidelity | Every surface form survives lowering and runtime consistently | Currently the main weakness |
| Static semantics | Type, trait, pattern, and module rules are coherent and diagnosable | Partially there |
| Runtime model | Values and effects are represented as first-class structures | Still string-heavy |
| Tooling fidelity | REPL, CLI, formatter, tests, and LSP all reflect the same language | Not converged yet |

For shell replacement, there is a second set of dimensions:

而要替代 Bash，还需要第二组维度：

| Dimension | What it means | Current pressure |
|-----------|---------------|------------------|
| Command model | Commands are typed objects, not shell strings | Minimal public constructor and execution bridges exist, but the legacy exec surface is still string/record-based |
| Stream model | Pipelines and redirects are first-class | Missing |
| Failure model | Exit codes, retries, cancellation, and timeouts are explicit | Mostly missing |
| Host interaction | Filesystem, env, cwd, and services are structured APIs | Partially started |
| Script entry model | Shebang, argv, and script packaging are defined | Missing |

## Validation Corpus / 验证语料

This roadmap is only meaningful if each phase is validated on real program classes.

如果没有真实样本，路线图会重新滑回“看起来完成”。因此每个阶段都要绑定验证语料。

### Language Validation Corpus / 语言验证语料

Every milestone should run against the following classes of Neve programs:

- module graph with `self`, `super`, `crate`, and re-export patterns
- algebraic data types plus deep pattern matching
- traits, methods, defaults, and associated types
- lazy evaluation plus tail recursion
- `Option` and `Result` heavy control flow
- records, updates, tuple indexing, safe field access, and list comprehension
- REPL, `neve check`, `neve run`, and LSP all observing the same program semantics

### Shell Replacement Validation Corpus / Shell 替代验证语料

Neve should eventually be able to replace the following real script categories:

- local test runner scripts like [test.sh](../scripts/test.sh)
- installer scripts like [install.sh](../scripts/install.sh)
- CI command orchestration like [ci.yml](../.github/workflows/ci.yml)
- package build wrappers and sandbox entrypoints
- host bootstrap and activation scripts
- service launch scripts and maintenance jobs

This gives a concrete way to answer "can Neve replace Bash yet?"

这会把“Neve 到底能不能替代 Bash”变成可验证问题，而不是主观判断。

## Replacement Levels / 替代层级

To avoid vague goals, Bash replacement is split into levels:

为了避免目标漂移，Bash 替代要分层定义：

| Level | Meaning | Included in roadmap |
|-------|---------|---------------------|
| L0 | Run commands and touch files | Yes, already started |
| L1 | Write robust non-interactive automation scripts | Yes, core target |
| L2 | Replace common CI/bootstrap/install scripts | Yes, core target |
| L3 | Replace most service/provisioning entry scripts | Yes, later target |
| L4 | Replace interactive user shell experience | No, not near-term |
| L5 | Reproduce full POSIX/Bash corner-case behavior | No, explicit non-goal |

Neve only needs L1-L3 to be a credible system-level language.
It does not need L4-L5 to be successful.

Neve 要成为可信的系统级语言，做到 L1-L3 就够了，不需要追求 L4-L5。

## Concrete Capability Matrix / 具体能力矩阵

### Standalone Language Matrix / 独立语言能力矩阵

| Capability | What "done" means | Main blockers today |
|------------|-------------------|---------------------|
| Canonical execution pipeline | `eval`, `run`, `check`, tests, and LSP all reflect one semantic core | AST and HIR paths diverge |
| Pattern fidelity | `or`, binding, rest, record patterns survive lowering intact | Lowering is lossy |
| Trait method semantics | Method calls, trait resolution, evaluation, and tooling agree on one dispatch order, and missing-method call sites report a dedicated diagnostic when no callable fallback exists | Long-term callable-fallback policy is still not fully settled |
| Error propagation | `Try`, `Option`, `Result`, and non-local failure rules are explicit and stable | Current handling differs by path |
| Path semantics | Path literals are typed values with stable operations | Paths still collapse into `String` |
| Match diagnostics | Exhaustiveness and unreachable arms are compiler-grade | Diagnostic hooks exist but are not wired in |
| Tooling fidelity | REPL `:type`, LSP, formatter, CLI all match the language | Tooling still contains partial semantic mismatches |
| Real end-to-end tests | Full pipeline tests execute the real runtime | Smoke baseline exists, but corpus breadth is still limited |

### Bash Replacement Matrix / Bash 替代能力矩阵

| Capability | What "done" means | Why it matters |
|------------|-------------------|----------------|
| Command object | Command is a typed value, not string interpolation | Avoids quoting bugs |
| Pipeline object | `cmd1 |> cmd2` is real process piping, not list piping reused accidentally | Core shell use case |
| Redirection model | stdin/stdout/stderr can be redirected to files, values, or pipelines | Necessary for scripts |
| Scoped env/cwd | Env and cwd can be set lexically for commands/tasks | Core automation need |
| Exit handling | Non-zero exit handling is explicit and composable | Reliable failure semantics |
| Timeout/cancel | Long-running processes can be bounded and stopped | Needed in CI and services |
| Signals/TTY | Interactive and service-facing commands behave predictably | Needed beyond toy scripts |
| Script entrypoint | Shebang, argv, and script packaging are defined | Replaces `.sh` entrypoints |
| Filesystem automation | Read/write/mkdir/remove/copy/glob have structured APIs | Removes shell fallback |
| Observability | Logs, captured output, and debug traces are structured | Required for maintainability |

## Work Package Catalog / 工作包目录

Each roadmap phase is implemented through concrete work packages.
These are the units that should be tracked in issues, milestones, and PRs.

路线图真正执行时，不应该按“大阶段”开工，而应该按工作包推进。
下面这些工作包才是 issue、里程碑和 PR 的最小可信单位。

| ID | Work package | Main crates | Depends on | Main output |
|----|--------------|-------------|------------|-------------|
| WP-0A | Feature support matrix | `docs`, `tests`, `neve-cli`, `neve-lsp` | None | Truthful support inventory |
| WP-0B | Real end-to-end harness | `tests`, `neve-frontend`, `neve-eval`, `neve-cli` | WP-0A | Real full-pipeline tests |
| WP-0C | Documentation status correction | `README`, `docs` | WP-0A | Honest project status |
| WP-1A | Pattern lowering fidelity | `neve-hir`, `neve-parser`, `neve-typeck`, `neve-eval` | WP-0A | No silent pattern loss |
| WP-1B | `Try` / `Option` / `Result` semantic unification | `neve-hir`, `neve-typeck`, `neve-eval`, `docs/reference/spec.md` | WP-0A | One failure-propagation model |
| WP-1C | Method call and trait dispatch unification | `neve-typeck`, `neve-hir`, `neve-eval` | WP-0A | One method semantics story |
| WP-1D | HIR evaluator parity for canonical language features | `neve-eval`, `neve-hir` | WP-1A, WP-1B, WP-1C | Canonical HIR runtime |
| WP-1E | Remove sentinel and placeholder semantic hacks | `neve-hir`, `neve-eval`, `neve-typeck` | WP-1D | Cleaner canonical runtime |
| WP-2A | Exhaustiveness checking | `neve-typeck`, `neve-eval/pattern`, `docs/reference/diagnostics.md` | WP-1A | Compiler-grade `match` coverage |
| WP-2B | Unreachable pattern analysis | `neve-typeck`, `docs/reference/diagnostics.md` | WP-2A | Reliable reachability warnings |
| WP-2C | Associated type use-site resolution | `neve-typeck`, `neve-hir`, `tests` | WP-1C | Traits usable beyond declaration checks |
| WP-2D | REPL and LSP semantic fidelity | `neve-cli`, `neve-lsp`, `neve-frontend` | WP-1D, WP-2A | Tooling matches language |
| WP-3A | Introduce `Path` runtime type | `neve-syntax`, `neve-hir`, `neve-typeck`, `neve-eval`, `neve-std` | WP-1D | Paths stop being strings |
| WP-3B | Introduce `Bytes` runtime type | `neve-eval`, `neve-std`, `docs/reference/api.md` | WP-3A | Binary-safe system APIs |
| WP-3C | Introduce `Command` and `ProcessResult` types | `neve-eval`, `neve-std`, `docs/reference/api.md` | WP-3A, WP-3B | Structured process model |
| WP-3D | Introduce pipeline and stream handles | `neve-eval`, `neve-std` | WP-3C | First-class process plumbing |
| WP-4A | Effect boundary design record | `docs/reference/spec.md`, `docs/contributor/architecture.md`, `docs` | WP-1D | Chosen effect model |
| WP-4B | Separate pure evaluation from task execution | `neve-eval`, `neve-cli`, `neve-config` | WP-4A | Pure/effectful split enforced |
| WP-4C | Audit and classify stdlib effects | `neve-std`, `docs/reference/api.md` | WP-4A | Stable effect taxonomy |
| WP-5A | First-class redirection runtime | `neve-std`, `neve-eval` | WP-3D, WP-4B | stdin/stdout/stderr composition |
| WP-5B | Scoped env/cwd and command context | `neve-std`, `neve-eval` | WP-3C, WP-4B | Safer script execution |
| WP-5C | Timeout, retry, cancellation, background jobs | `neve-std`, `neve-eval`, `neve-cli` | WP-5A, WP-5B | CI/service-grade scripting |
| WP-5D | Signal, TTY, and shebang entrypoint model | `neve-cli`, `neve-std`, `docs/reference/spec.md` | WP-5C | Real script replacement |
| WP-5E | Port validation corpus scripts to Neve | `scripts`, `examples`, `tests` | WP-5D | Proof of shell replacement |
| WP-6A | Lockfile and resolver integration | `neve-derive`, `neve-cli`, `docs` | WP-1D | Reproducible dependency story |
| WP-6B | Registry and package metadata standard | `neve-derive`, `neve-fetch`, `docs` | WP-6A | Shareable package ecosystem |
| WP-6C | Stdlib layering and stability policy | `neve-std`, `docs/reference/api.md`, `docs/project/changelog.md` | WP-4C | Stable language platform |
| WP-6D | Compatibility and release policy | `docs`, `README`, `release` workflow | WP-6B, WP-6C | Language lifecycle defined |

### Work Package Notes / 工作包注记

- `WP-0A` is mandatory before serious implementation decisions. Otherwise work will keep optimizing the wrong surface.
- `WP-1A` to `WP-1E` are the semantic convergence block. Without them, every later runtime feature rests on unstable meaning.
- `WP-3A` to `WP-5E` are the actual system-language block. This is where Neve starts earning the "replace shell scripts" claim.
- `WP-6A` to `WP-6D` are what make Neve a standalone language ecosystem rather than a repo-local language runtime.

## Decision Gates / 关键决策门

Some questions must be settled explicitly.
If they are left implicit, the implementation will keep drifting.

有些问题必须在路线图层面先定掉，不然实现会持续分叉。

### Gate G1: Canonical Pipeline / 决策门 G1：规范执行管线

Decision / 要决策的问题:

- Is HIR evaluation the canonical runtime, with AST evaluation downgraded to transitional tooling?

Required answer / 建议答案:

- Yes. The long-term semantic authority should be `Parser -> HIR -> Typeck -> HIR Eval`.

### Gate G2: Method Semantics / 决策门 G2：方法语义

Decision / 要决策的问题:

- Is `x.foo(y)` just sugar for `foo(x, y)`, or does it mean trait-based method dispatch?

Status: ✅ Resolved (PR-016). The canonical dispatch order is: inherent impl → trait method → callable-target fallback `foo(x, y)`. This precedence is test-locked. When neither method nor callable target exists, a dedicated `UnknownMethod` diagnostic is emitted. The two separate impl-assoc resolution paths have been collapsed into one canonical `canonical_impl_assoc_types` helper (D-070).

Required answer / 建议答案:

- Current canonical rule: `x.foo(y)` first attempts receiver method dispatch, then falls back to the lowered callable form `foo(x, y)` if no method resolves. ✅ Done.
- Next step: keep that precedence explicit, tested, and documented before considering any future breaking change to remove the fallback branch. ✅ Done.

### Gate G3: Failure Propagation / 决策门 G3：失败传播

Decision / 要决策的问题:

- What exactly does `?` mean for `Result`, `Option`, and effectful task execution?

Status: ✅ Resolved (PR-017). The optional-flow semantics are now unified through `resolve_optional_flow_payload` for `?`, `??`, and `?.`. The closure matrix covers builtin Option/Result, user enum `Some/None`/`Ok/Err`, record safe-field access, and invalid-use rejection. All tooling surfaces (frontend, REPL, LSP) share the same diagnostics. Effectful task execution (`Task<T>`) remains a separate concern deferred to Gate G4 / PR-011.

Required answer / 建议答案:

- `?` must have one rule family, documented in the spec, implemented identically across all execution paths. ✅ Done for the pure-language subset.

### Gate G4: Effect Boundary / 决策门 G4：副作用边界

Decision / 要决策的问题:

- Are effects embedded directly in normal evaluation, or represented through a task/command layer?

Required answer / 建议答案:

- Keep the core language pure and make effects explicit through a dedicated execution layer.

### Gate G5: Bash Replacement Scope / 决策门 G5：Bash 替代范围

Decision / 要决策的问题:

- Is the goal full POSIX shell compatibility, or structured replacement of common shell workloads?

Required answer / 建议答案:

- Structured replacement of common shell workloads. Full POSIX behavior compatibility is not the near-term target.

## Workstreams / 工作流

### 1. Reality Alignment / 现实校准

Goal / 目标:
Make the documented status match the actual implementation boundary.

Deliverables / 交付物:

- feature matrix covering parser, lowering, typeck, eval, fmt, LSP, tests
- documentation cleanup for overstated "complete" claims
- removal or relabeling of placeholder end-to-end coverage

Exit criteria / 退出标准:

- every syntax feature has an explicit support matrix
- every "complete" label is defensible
- end-to-end tests exercise the real pipeline

### 2. Canonical Pipeline Convergence / 规范语义管线收敛

Goal / 目标:
Choose one canonical language pipeline and make it authoritative.

Required direction / 建议方向:

- canonical path: `Parser -> HIR lowering -> Type checking -> HIR evaluation`
- AST evaluator becomes bootstrap or compatibility tooling, not the primary semantic authority

Deliverables / 交付物:

- parity list between AST evaluator and HIR evaluator
- elimination of lossy lowering
- removal of unsupported-expression placeholders from canonical execution

Exit criteria / 退出标准:

- `neve eval`, `neve run`, and `neve check` all describe and execute the same language
- HIR evaluation supports the canonical feature set
- behavior no longer depends on whether a command took the AST or HIR path

### 3. Type and Pattern Closure / 类型系统与模式语义闭环

Goal / 目标:
Make pattern matching, traits, and error propagation fully coherent.

Missing areas / 缺失点:

- non-exhaustive match checking
- unreachable pattern detection
- faithful lowering for `or`, binding, and rest patterns
- method dispatch semantics aligned with trait resolution
- associated type usage made real across checking and evaluation
- `Try` and `Result` propagation semantics stabilized

Exit criteria / 退出标准:

- all pattern forms in the spec survive lowering without semantic loss
- trait methods have one consistent dispatch model
- match diagnostics are sound enough for compiler-grade use

### 4. Core Runtime Model / 核心运行时模型

Goal / 目标:
Promote system values from strings to structured runtime types.

Required value models / 必要的值模型:

- `Path`
- `Bytes`
- `Command`
- `ProcessResult`
- `Env`
- `Stream` or equivalent pipeline handle
- `Duration` and timeout model

Exit criteria / 退出标准:

- path literals no longer degrade to plain strings
- process execution no longer relies on unstructured string conventions

Current state / 当前状态:

- internal `Path` identity exists and `std.path.fromString` plus a minimal typed `std.path` adapter set (`path.joinPath`, `path.parentPath`, `path.filenamePath`, `path.extensionPath`, `path.isAbsolutePath`) expose it to user code, with `std.io.readFilePath`, `std.io.readDirPath`, `std.io.readDirEntryPaths`, `std.io.writeFilePath`, `std.io.appendFilePath`, `std.io.currentDirPath`, `std.io.homeDirPath`, `std.io.createDirAllPath`, `std.io.removeDirAllPath`, `std.io.hashFilePath`, `std.io.pathExistsPath`, and `std.io.isFilePath` / `std.io.isDirPath` as the first typed host-boundary bridges; `io.readDirPath` intentionally still projects to `List<String>` entry names for compatibility, while `io.readDirEntryPaths` is the explicit `List<Path>` directory-entry bridge
- internal `Bytes` identity exists and `std.io.readFileBytesPath : Path -> Bytes` plus `std.io.writeFileBytesPath : Path -> Bytes -> Unit` / `std.io.appendFileBytesPath : Path -> Bytes -> Unit` now expose the first minimal public host bridges for binary-safe file input/output, but broader bytes-oriented stdlib APIs are still intentionally absent
- dormant internal `Command` / `ProcessResult` identities now exist in `neve-eval`, and `typeOf` / runtime formatting / stable keys recognize them as first-class runtime objects
- dormant internal `Pipeline` identity now also exists in `neve-eval`, `std.io.pipeline` exposes a first minimal public `List<Command> -> Pipeline` constructor bridge and now rejects obviously invalid pipeline objects at construction time (empty pipeline, `non-final stdout` redirect, `non-first stdin`, `non-first stdin` redirect), `std.io.pipelineWithRedirects` lets a public `Pipeline` carry boundary `List<Redirect>` while rejecting obvious boundary-vs-stage-local conflicts, and `std.io.execPipeline` now provides the first blocking buffered `Pipeline -> ProcessResult` execution bridge while boundary redirects are carried on the object itself rather than a separate wrapper API
- dormant internal `Redirect` identity now also exists in `neve-eval`, and `std.io.redirectStdoutPath`, `std.io.redirectStderrPath`, plus `std.io.redirectStdinPath` expose the first minimal public `Path -> Redirect` constructor bridges without claiming a general stream model
- `io.commandWithRedirects` and `io.pipelineWithRedirects` now expose the minimal public redirect attachment bridges for command/pipeline execution, and let the same `stdout -> Path`, `stderr -> Path`, plus `stdin <- Path` redirect objects live on first-class `Command` / `Pipeline` objects without implying general stream composition
- dormant internal `Task<T>` identity now also exists in `neve-eval`, and `std.io.taskCommand` plus `std.io.taskPipeline` expose the first minimal public `Command|Pipeline -> Task[ProcessResult]` constructor bridges without claiming scheduling, await, or cancellation semantics yet
- `std.io.awaitTask` and `std.io.awaitTasks` now expose the first minimal public `Task[ProcessResult] -> ProcessResult` and `List<Task[ProcessResult]> -> List<ProcessResult>` consumer bridges for command-backed and pipeline-backed tasks, but they are still only blocking canonical boundary ops, not a scheduler, poller, or cancellation model
- `std.io.command` and `std.io.execCommand` now expose the first minimal public `Command -> ProcessResult` bridge, `std.io.commandWith` lets a public `Command` carry `cwd` / `env` / `stdin` configuration, and `std.io.commandWithRedirects` now also lets a public `Command` carry a typed `List<Redirect>` so stage-local redirect attachment can start without inventing a second command runtime; pipeline construction now rejects explicit object/topology errors including empty pipelines, final-stage boundary `stdout/stderr` conflicts, non-final stage `stdout` redirects, and non-first stage `stdin` or `stdin` redirects instead of silently deferring them to execution
- the transitional `io.execResult` / `io.execShellResult` / `io.execWithResult` aliases have now been removed, and the old plain `io.exec`, shell `io.execShell`, plus configured `io.execWith` wrappers are gone as well, leaving no public compatibility execution surface outside the object-carried mainline
- `std.io.processSuccess`, `std.io.processStdout`, `std.io.processCode`, and `std.io.processStderr` now expose the first pure `ProcessResult` inspector bridges; `io.command`, `io.commandWith`, `io.commandWithRedirects`, `io.execCommand`, `io.pipeline`, `io.pipelineWithRedirects`, and `io.execPipeline` now carry the public canonical execution story, explicit shell behavior is expressed by constructing `io.command("sh"/"cmd", ["-c"/"/C", ...])`, `io.redirectStdoutPath` / `io.redirectStderrPath` / `io.redirectStdinPath` participate through those first-class command/pipeline objects, and `io.taskCommand` / `io.taskPipeline` plus `io.awaitTask` / `io.awaitTasks` now give `Task[ProcessResult]` a first public single/bulk construction-consumption loop across command and pipeline boundaries, so the remaining `WP-3C` / `WP-3D` / `WP-5A` gap is concentrated in richer streaming semantics beyond current command/pipeline boundary composition, broader stage-local composition control, and non-blocking task execution/control
- path literals and most stdlib contracts are still string-based, so `WP-3A` is started but not closed
- shell-facing operations become composable library primitives

### 5. Effect Boundary / 副作用边界

Goal / 目标:
Separate pure evaluation from effectful execution clearly.

Options / 可选设计方向:

- a typed effect system
- a dedicated `Task` or `Command` layer interpreted by a runtime
- a staged evaluation model: pure config phase, effectful execution phase

Minimum requirement / 最低要求:

- pure expressions remain referentially transparent
- I/O and process execution are explicit in the language model
- configuration evaluation does not silently perform host mutations

Exit criteria / 退出标准:

- the docs' "pure functional core" claim becomes technically true
- effectful code is visible and auditable

### 6. Shell Replacement Capability Layer / 替代 Bash 的能力层

Goal / 目标:
Cover shell usage with typed, structured primitives.

Capabilities still needed / 仍需补齐的能力:

- first-class pipelines without going through shell parsing ✅
- redirection model for stdin/stdout/stderr ✅
- stream plumbing between processes ✅ (Stream<T> 14 APIs, Phase A-C complete)
- background job and cancellation model ✅ (spawn/poll/cancel/awaitAny ready)
- exit code policy and failure composition ✅
- working directory and scoped environment mutation ✅
- globbing or explicit directory/query combinators ✅
- signal handling and TTY-aware execution ⚠️ (signals done, raw TTY pending)
- shebang and argv handling for script entrypoints ✅

Important note / 重要说明:

Replacing Bash does **not** mean duplicating every POSIX corner case.
It means offering a safer and more structured alternative for the jobs people actually use Bash for.

### 7. Standard Library and Ecosystem Closure / 标准库与生态闭环

Goal / 目标:
Make Neve usable as a real language, not just a language core.

Missing platform / 缺失平台:

- package metadata standard
- deterministic lockfile flow
- registry conventions
- stable stdlib layering
- test runner and golden test conventions
- release and compatibility policy

Exit criteria / 退出标准:

- Neve programs can depend on versioned libraries in a reproducible way
- stdlib modules have clear stability and ownership boundaries

## Phase Plan / 分阶段计划

### Phase 0: Reality Alignment / 阶段 0：现实校准

| Item | Details |
|------|---------|
| Priority | Highest |
| Work packages | `WP-0A`, `WP-0B`, `WP-0C` |
| Entry criteria | None |
| Main output | Truthful support matrix and real end-to-end baseline |
| Why this phase exists | Later implementation is worthless if the project is measuring the wrong thing |
| Blocks later phases | Yes, this phase blocks every serious semantic decision |

Detailed scope / 详细范围:

- enumerate every syntax form and semantic feature from spec, parser, AST, HIR, evaluator, stdlib, REPL, LSP, and tests
- build a matrix with columns: parse, lower, typeck, HIR eval, AST eval, fmt, LSP, tests, docs
- keep expanding the real executable full-pipeline tests so they track the language surface
- correct optimistic status labels in docs and README

Validation / 验证:

- the matrix exists in-repo and is reviewable
- `tests/end_to_end.rs` no longer relies on placeholder execution
- no "complete" claim remains unsupported by the matrix

Main risks / 主要风险:

- this phase may reveal that the current complete surface is smaller than expected
- some docs and tests may need to be downgraded before anything looks better

Non-goals / 非目标:

- adding new syntax
- optimizing runtime
- redesigning effects before the factual baseline is clear

### Phase 1: Semantic Convergence / 阶段 1：语义收敛

| Item | Details |
|------|---------|
| Priority | Highest |
| Work packages | `WP-1A`, `WP-1B`, `WP-1C`, `WP-1D`, `WP-1E` |
| Entry criteria | Phase 0 complete |
| Main output | One canonical semantic story from parser to runtime |
| Why this phase exists | Neve currently has a broad syntax surface but multiple semantic paths |
| Blocks later phases | Yes, type-system hardening and effect design both depend on this |

Detailed scope / 详细范围:

- fix lossy lowering for `or` patterns, binding patterns, rest patterns, and any fallback-to-wildcard behavior
- define the exact semantics of `Try`, `Option`, `Result`, `Coalesce`, and method calls
- either bring HIR evaluator to feature parity for the canonical language or explicitly lower unsupported constructs before runtime
- remove placeholder and sentinel semantic hacks from canonical execution paths

Validation / 验证:

- the same source program behaves consistently in `neve eval`, `neve run`, and end-to-end tests
- no feature is accepted by parser but silently degraded during lowering
- method syntax and failure propagation semantics are documented and tested

Main risks / 主要风险:

- some AST-evaluator-only behavior may need to be removed or redefined
- compatibility with existing examples may break where semantics were previously accidental

Non-goals / 非目标:

- advanced effect runtime
- shell pipelines
- package ecosystem work

### Phase 2: Compiler-Grade Type Semantics / 阶段 2：编译器级类型语义

| Item | Details |
|------|---------|
| Priority | High |
| Work packages | `WP-2A`, `WP-2B`, `WP-2C`, `WP-2D` |
| Entry criteria | Phase 1 complete |
| Main output | Type checker becomes the trusted semantic validator |
| Why this phase exists | A complete language needs compiler-grade diagnostics, not just local inference |
| Blocks later phases | It blocks stable language claims and high-confidence tooling |

Detailed scope / 详细范围:

- implement exhaustiveness checking for `match`
- emit unreachable-pattern warnings with stable diagnostics
- resolve associated types at real use sites, not only in declarations and completeness checks
- make REPL `:type`, LSP hover, and CLI type checking reflect the same resolved semantics

Validation / 验证:

- missing match arms are diagnosed with reproducible diagnostics
- unreachable arms are warned consistently
- associated type examples run through check, eval, and tooling without semantic divergence

Main risks / 主要风险:

- exhaustiveness algorithms can become complex if pattern lowering remains unstable
- tooling may expose unresolved type holes that were previously hidden

Non-goals / 非目标:

- effect runtime
- shell replacement APIs
- registry and lockfile work

### Phase 3: Effect and Runtime Layer / 阶段 3：副作用与运行时层 ✅ Complete

| Item | Details |
|------|---------|
| Priority | High |
| Work packages | `WP-3A`, `WP-3B`, `WP-3C`, `WP-3D`, `WP-4A`, `WP-4B`, `WP-4C` |
| Status | **Complete** — runtime types, EffectEval v4.3 (34 rules), effect-boundary.md v1.0 |
| Main output | Typed runtime objects plus explicit effect boundary |
| Entry criteria | Phase 1-2 complete |
| Blocks later phases | Yes, shell replacement depends on this phase |

Detailed scope / 详细范围:

- introduce runtime types for `Path`, `Bytes`, `Command`, `ProcessResult`, and pipeline handles
- write and accept an effect-boundary design record
- classify stdlib APIs into pure vs effectful layers
- prevent pure configuration evaluation from silently becoming host-mutating script execution

Validation / 验证:

- path literals become real path values
- process execution can be expressed without ad hoc strings
- docs clearly distinguish pure evaluation from task execution

Main risks / 主要风险:

- touching the value model will ripple through evaluator, stdlib, docs, and tests
- weak effect design here will cause expensive refactors later

Non-goals / 非目标:

- full shell replacement
- job control
- package registry

### Phase 4: Shell Capability Replacement / 阶段 4：Shell 能力替代

| Item | Details |
|------|---------|
| Priority | High |
| Work packages | `WP-5A`, `WP-5B`, `WP-5C`, `WP-5D`, `WP-5E` |
| Status | **Complete** ✅ — Stream<T> 14 APIs; 537 E2E tests; Formatter 37/37; Clippy 0 warnings |
| Entry criteria | Phase 3 complete |
| Main output | Typed scripting runtime that covers common shell workloads |
| Why this phase exists | `exec` builtins alone do not replace Bash |
| Blocks later phases | It blocks any serious "system-level language" claim |

Detailed scope / 详细范围:

- first-class redirections for stdin/stdout/stderr ✅
- first-class pipelines and stream composition ✅
- scoped env/cwd execution contexts ✅
- retries, timeouts, cancellation, and background execution ⚠️ (timeout/retry done, cancel pending)
- signal handling, TTY-aware execution, and shebang/argv entrypoints ⚠️ (signals + shebang done, TTY pending)
- port the validation corpus scripts to Neve as proof, not just as aspiration

**Completed (Phase 4 current)**:
- `|>` pipe syntax: cmd1 |> cmd2 → Pipeline (AST + HIR parity) ✅
- Pipeline construction + execution (blocking + streaming + timeout) ✅
- Redirect boundary validation + stage-local redirects ✅
- Task<T> spawn/poll/awaitWithTimeout lifecycle ✅
- Signal handlers (5 signals: INT/TERM/HUP/USR1/USR2) ✅
- Glob pattern matching ✅
- Shebang + io.args() structured parsing ✅
- E2E tests: 440 ✅
- Formatter idempotency: 37/37 ✅
- Stream<T> Phase A-C: 14 APIs (construct + transform + consume + pipe) ✅
- EffectEval v4.3 (34 rules: +5 stream Phase C) ✅
- io.cancel + io.awaitAny ✅
- TTY APIs: isTTY/terminalSize/setRawMode/resetTerminal ✅
- Job control: io.jobs/io.waitAnyJob ✅
- Clippy: 0 warnings ✅
- Example scripts: test-runner.neve, ci-bootstrap.neve ✅

**In Design / Pending**:
- TTY raw mode: pending (API exists, full integration pending)

Validation / 验证:

- Neve versions of local test runner, installer, and representative CI task scripts exist
- common automation no longer falls back to Bash for piping, redirects, or env scoping
- script failure behavior is explicit and testable

Main risks / 主要风险:

- interactive shell expectations may creep in and expand scope
- stream and process semantics can become platform-sensitive

Non-goals / 非目标:

- full POSIX shell compatibility
- interactive shell replacement
- shell quoting emulation as a design goal

### Phase 5: Ecosystem Completion / 阶段 5：生态补完 ⬅️ Current

| Item | Details |
|------|---------|
| Priority | Medium (current active phase) |
| Work packages | `WP-6A`, `WP-6B`, `WP-6C`, `WP-6D` |
| Status | **In Progress** — Flake/lock/store infra done; stability tiers defined; package CLI exists |
| Entry criteria | Phases 1-3 complete; Phase 4 recommended |
| Main output | Reproducible package and library ecosystem |
| Why this phase exists | A standalone language needs more than syntax and runtime |
| Blocks later phases | It blocks real external adoption |

Detailed scope / 详细范围:

- integrate deterministic lockfiles with dependency resolution ✅ (flake.neve + flake.lock implemented)
- define registry and package metadata format ⚠️ (store format exists; package index pending)
- define stdlib stability tiers and compatibility rules ✅ (see `docs/reference/stability.md`)
- define release, compatibility, and deprecation policy ✅ (see `docs/reference/stability.md`)

**Completed (Phase 5 so far)**:
- Flake system: `flake.neve` + `flake.lock` with GitHub/Git/URL/local inputs ✅
- Content-addressed store (`/neve/store`) with NAR archives + Ed25519 signatures ✅
- Garbage collection with generation-based roots ✅
- Binary cache / substituter support ✅
- Package CLI: `neve package install/remove/list/rollback` ✅
- `neve search <query>` ✅
- `neve build` with native/Docker/simple backends ✅
- `neve update` ✅
- `neve config` with generation-based system config (build/switch/rollback/list/verify) ✅
- Profile generations with atomic rollback ✅
- Stdlib stability tiers defined (Tier 1/2/3) ✅
- Ecosystem design doc created (`docs/project/ecosystem-design.md`) ✅

**In Design / Pending**:
- Package registry / index (central package discovery)
- Module system ↔ flake integration (import resolving flake inputs)
- Cross-platform package management (currently Unix-only)

Validation / 验证:

- versioned Neve packages can be resolved reproducibly ✅
- stdlib changes follow explicit compatibility policy ✅
- releases communicate language and library stability clearly ✅

Main risks / 主要风险:

- ecosystem work will magnify any unresolved semantic instability
- releasing too early will freeze bad abstractions

Non-goals / 非目标:

- speculative advanced type features like HKT before the core is stable
- broad package ecosystem expansion before metadata and compatibility rules exist

## Execution Order / 执行顺序

The intended order is:

预期执行顺序如下：

1. `WP-0A -> WP-0B -> WP-0C`
2. `WP-1A -> WP-1B -> WP-1C -> WP-1D -> WP-1E`
3. `WP-2A -> WP-2B -> WP-2C -> WP-2D`
4. `WP-3A -> WP-3B -> WP-3C -> WP-3D`
5. `WP-4A -> WP-4B -> WP-4C`
6. `WP-5A -> WP-5B -> WP-5C -> WP-5D -> WP-5E`
7. `WP-6A -> WP-6B -> WP-6C -> WP-6D`

Two constraints are especially important:

- Do not start shell replacement work (`WP-5*`) before the effect boundary is settled (`WP-4*`).
- Do not start ecosystem freezing (`WP-6*`) before semantic convergence is complete (`WP-1*` and `WP-2*`).

## Explicit Deferrals / 明确延后项

The following items are intentionally **not** near-term priorities:

下列事项明确不是近期优先级：

| Deferred item | Why deferred |
|---------------|--------------|
| Macros | They amplify semantic instability if introduced too early |
| HKT / higher-kinded types | Not justified before core type semantics are complete |
| FFI | Needs a stable runtime and effect boundary first |
| Interactive shell replacement | Larger scope than non-interactive system scripting |
| Full POSIX compatibility | High cost, low strategic value for Neve |
| Advanced concurrency model | Needs effect runtime and process model first |

## Progress Metrics / 进度度量

Each phase should be tracked with concrete metrics, not only narrative progress.

每个阶段都应该有可以量化的进度指标：

| Metric | Meaning |
|--------|---------|
| Feature matrix coverage | Percentage of syntax/semantic items classified across parser/lowering/typeck/eval/tooling |
| Real E2E coverage | Number of end-to-end programs executed through the real pipeline |
| Lossy lowering count | Number of language constructs that degrade or fall back during lowering |
| Placeholder count | Number of placeholder tests, placeholder statuses, and runtime semantic hacks |
| Stringly system API count | Number of system-facing APIs still encoded primarily as raw strings |
| Script port count | Number of real shell scripts replaced by Neve implementations |

## What Must Not Happen Next / 接下来不该做的事

- Do not keep adding new syntax while current lowering is lossy.
- Do not claim language completion just because a smoke E2E baseline exists.
- Do not treat shell replacement as just "add `exec` builtins".
- Do not let pure configuration evaluation silently become an effectful scripting runtime.
- Do not keep both AST and HIR semantics drifting independently.

## Immediate Priority Order / 当前立即优先级

Updated 2026-06-03. Phase 4-6 are complete. Current priorities align with remaining decision chain (Q8→Q9→Q5→Q4→Q6/Q7):

2026-06-03 更新。Phase 4-6 已完成。当前优先级与剩余决策链对齐：

1. **Q8: AST evaluator deprecation** — v3.1 目标: 在 `neve_eval::compat` 路径添加 deprecation 警告
2. **Q9: Parser gap closure** — 补齐 3 个已知 gap (crate:: import, 多行注释, tuple index)
3. **Q5: 发布策略** — v3.2 目标: 正式化 SemVer 混合策略
4. **Lean 验证** — 等待 Lean 4.29+ 解除 mutual inductive 限制
5. **Q4: Windows** — 评估 Tiered support model (Tier 1: Linux, Tier 2: macOS, Tier 3: Windows)
6. **Q6: Registry** — 内部验证后公开

## Next Execution Batch / 下一执行批次

Updated 2026-06-03.

| Priority | Task | Files | Target |
|----------|------|-------|--------|
| 1 | AST evaluator deprecation 警告 | `crates/neve-eval/src/compat/` | v3.1 |
| 2 | Parser gap 补齐 (crate::, 多行注释, tuple index) | `crates/neve-parser/`, `crates/neve-lexer/` | v3.1 |
| 3 | Release policy 文档化 | `docs/reference/stability.md`, `docs/project/changelog.md` | v3.2 |
| 4 | Lean 技能套装拉取 + 剩余 axioms 监控 | `formal/` | ongoing |
| 5 | Windows 支持评估 | `crates/neve-builder/`, `crates/neve-config/`, `.github/workflows/` | v3.x |
| 6 | Registry 内部验证 + 公开策略 | `crates/neve-fetch/`, `crates/neve-store/`, docs | v3.x |

## Acceptance Standard / 验收标准

Neve can be called a complete standalone language only when:

- one canonical pipeline defines semantics
- tests cover that real pipeline
- docs describe the real boundary
- stdlib and runtime models are structured and stable
- tooling reflects actual semantics

Neve can be called a Bash replacement layer only when:

- commands, pipelines, redirects, environments, and failures are first-class
- common system automation tasks no longer require Bash escape hatches
- the effect model is explicit enough for system configuration use
