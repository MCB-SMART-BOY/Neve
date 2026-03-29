<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Language Completion Roadmap</h1>

<p><em>语言完备化与系统脚本化路线图</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong> ·
  <strong><a href="roadmap.md">Project Roadmap</a></strong>
</p>

</div>

---

This document is the focused roadmap for turning Neve into:

- a complete standalone programming language
- a system-level scripting language that can replace most shell usage

这份文档专门回答两个目标：

- 把 Neve 做成一个独立完备的编程语言
- 把 Neve 做成一个能替代大部分 shell 用途的系统级脚本语言

It is intentionally stricter than the general project roadmap.
The main rule is simple:

**Do not keep adding surface syntax until the existing syntax has one coherent semantic pipeline.**

核心约束也很简单：

**在现有语法没有形成单一、闭环、可验证的语义管线之前，不再继续优先扩张表面语法。**

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

- first-class pipelines without going through shell parsing
- redirection model for stdin/stdout/stderr
- stream plumbing between processes
- background job and cancellation model
- exit code policy and failure composition
- working directory and scoped environment mutation
- globbing or explicit directory/query combinators
- signal handling and TTY-aware execution
- shebang and argv handling for script entrypoints

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

Priority / 优先级:
Highest.

Immediate tasks / 立即任务:

1. Build the feature matrix for every syntax form and major semantic feature.
2. Replace placeholder end-to-end tests with real pipeline tests.
3. Downgrade or correct misleading "complete" status claims.
4. Mark the AST evaluator as transitional if it remains more capable than HIR evaluation.
5. Freeze nonessential syntax expansion until the matrix exposes the true gaps.

Exit criteria / 退出标准:

- the project can state precisely what Neve supports today
- no major feature is "supported" only in parser or only in AST evaluation without being called out

### Phase 1: Semantic Convergence / 阶段 1：语义收敛

Priority / 优先级:
Highest.

Immediate targets / 近期目标:

- fix lossy lowering for pattern forms
- unify method semantics
- stabilize `Try`, `Option`, and `Result` behavior
- bring HIR evaluator to parity for canonical language features

Exit criteria / 退出标准:

- one semantic story from parse to runtime
- no silent feature degradation during lowering

### Phase 2: Compiler-Grade Type Semantics / 阶段 2：编译器级类型语义

Priority / 优先级:
High.

Targets / 目标:

- exhaustiveness checking
- unreachable pattern warnings
- stronger trait diagnostics
- associated type resolution across use sites

Exit criteria / 退出标准:

- the type checker is credible as the main semantic validator

### Phase 3: Effect and Runtime Layer / 阶段 3：副作用与运行时层

Priority / 优先级:
High.

Targets / 目标:

- define `Path`, `Command`, `ProcessResult`, `Env`
- formalize effectful execution boundary
- stop using plain strings as the long-term system-language interface

Exit criteria / 退出标准:

- system programming primitives become typed and composable

### Phase 4: Shell Capability Replacement / 阶段 4：Shell 能力替代

Priority / 优先级:
High.

Targets / 目标:

- pipeline API
- redirect API
- background jobs
- timeout and cancellation
- signal handling
- script entrypoint model

Exit criteria / 退出标准:

- common automation scripts can be written directly in Neve without falling back to Bash

### Phase 5: Ecosystem Completion / 阶段 5：生态补完

Priority / 优先级:
Medium.

Targets / 目标:

- versioned package story
- lockfiles
- registry
- compatibility and release policy

Exit criteria / 退出标准:

- Neve is usable as a self-hosted language ecosystem, not only as a project-local tool

## What Must Not Happen Next / 接下来不该做的事

- Do not keep adding new syntax while current lowering is lossy.
- Do not claim language completion while end-to-end tests are placeholder-based.
- Do not treat shell replacement as just "add `exec` builtins".
- Do not let pure configuration evaluation silently become an effectful scripting runtime.
- Do not keep both AST and HIR semantics drifting independently.

## Immediate Priority Order / 当前立即优先级

1. Feature matrix and status correction
2. Real end-to-end pipeline tests
3. Pattern lowering fidelity
4. Canonical `Try` and error propagation semantics
5. Trait method dispatch unification
6. HIR evaluator parity for canonical features
7. Exhaustiveness and unreachable-pattern diagnostics
8. `Path` and `Command` value model
9. Explicit effect boundary
10. First-class pipeline and redirection runtime

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
