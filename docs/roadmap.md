```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                                 NEVE ROADMAP                                 ║
║                                   路线图                                     ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  Constraints / Phases / Syntax Policy             │
│  [中文]     #chinese   ──→  约束 / 阶段 / 语法变更政策                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

<a name="english"></a>

# English

This roadmap turns the "inherit and surpass" vision into a phased execution plan.
Neve is a from-scratch language. Syntax decisions are not constrained by Nix
compatibility. The guiding constraints below are mandatory for all phases.

## Guiding Constraints

- From-scratch syntax: no compatibility targets with nixpkgs or Nix expressions.
- Zero ambiguity: every syntax form must parse to exactly one meaning.
- Syntax unity: similar concepts use similar syntax and semantics.
- Explicit delimiters: no indentation-sensitive parsing.
- Pure functional core: effects must be explicit and isolated from evaluation.

## Phase A: Core Language Stability

Goal: finish the in-progress core and freeze the language surface for v1.

Work items
- Complete module loader (content-addressed cache and invalidation rules).
- Associated types in the trait system.
- Tail-call optimization correctness and limits.
- LSP essentials: diagnostics, hover, go-to-def, basic completion.
- Error reporting polish and formatter stability.

Exit criteria
- Spec v2.0 frozen and referenced by parser/typeck/eval tests.
- Parser ambiguity tests and syntax change checklist in place.
- LSP baseline experience: parse + type diagnostics within a single file.
- Formatter is idempotent across the test corpus.

## Phase B: Package Manager MVP

Goal: a minimal, reliable build graph with reproducible outputs and locking.

Work items
- Dependency resolver with deterministic lockfile format.
- Fetch cache and verification pipeline (URL, git, local).
- Store integration with GC and content-addressed validation.
- Builder lifecycle: inputs -> sandbox -> outputs -> store registration.
- CLI flows: build/search/update basics.

Exit criteria
- Build a package twice and get identical store paths.
- Lockfile ensures repeatable resolution across machines.
- GC can free unreachable paths safely.

## Phase C: Configuration System MVP

Goal: typed, composable system configuration with generations and rollback.

Work items
- Strongly typed options for modules and configuration merge semantics.
- Module imports with deterministic ordering and conflict resolution.
- Generation management: build, switch, rollback, list.
- Minimal host config coverage: users, services, environment, packages.

Exit criteria
- A minimal system config can be built, activated, and rolled back.
- Module option typing catches errors at compile time.

## Phase D: Ecosystem and Beyond

Goal: a complete distribution pipeline and contributor-friendly ecosystem.

Work items
- Binary cache and substituter protocol (NAR-based).
- Registry and package metadata standard.
- Standard library growth with stability guarantees.
- Documentation, tutorials, and contribution workflows hardened.

Exit criteria
- Reproducible binary distribution with verified substitutes.
- Contributor workflow documented and automated.

## Syntax Change Policy

All syntax changes must:
- Update `docs/spec.md` first.
- Pass parser ambiguity and golden syntax tests.
- Update formatter, LSP tokens, and diagnostics together.
- Avoid "compatibility" hacks that increase ambiguity.

---

<a name="chinese"></a>

# 中文

这份路线图把“继承并超越”的愿景落实为分阶段的执行计划。
Neve 是一门从头设计的语言，语法决策不受 Nix 兼容性约束。
以下指导约束在所有阶段都必须遵守。

## 指导约束

- 从头设计语法：不以兼容 nixpkgs 或 Nix 表达式为目标。
- 零歧义：每一种语法形式只允许一种解析结果。
- 语法统一：相似概念使用相似的语法与语义。
- 显式分隔：不使用缩进敏感解析。
- 纯函数核心：副作用必须显式隔离，不能混入求值过程。

## 阶段 A：语言核心稳定

目标：完成进行中的核心能力，并冻结 v1 语言表面。

工作项
- 完成模块加载器（内容寻址缓存与失效规则）。
- Trait 系统的关联类型。
- 尾调用优化的正确性与边界。
- LSP 基础能力：诊断、悬停、跳转、基础补全。
- 错误信息与格式化器稳定。

退出标准
- 规范 v2.0 冻结，并由 parser/typeck/eval 测试覆盖。
- 解析歧义测试与语法变更清单到位。
- LSP 基线体验：单文件内可给出解析与类型诊断。
- 格式化器对测试语料幂等。

## 阶段 B：包管理 MVP

目标：最小可用、可靠的构建图，具备可复现输出与锁定。

工作项
- 依赖解析器与确定性的 lockfile 格式。
- Fetch 缓存与校验流水线（URL、git、本地）。
- Store 与 GC、内容寻址校验的整合。
- 构建生命周期：输入 -> 沙箱 -> 输出 -> Store 注册。
- CLI 基本流程：build/search/update。

退出标准
- 同一包构建两次得到相同的 store 路径。
- lockfile 能跨机器复现解析结果。
- GC 能安全清理不可达路径。

## 阶段 C：配置系统 MVP

目标：带类型的可组合系统配置，支持代际与回滚。

工作项
- 模块选项强类型与配置合并语义。
- 模块导入的确定性顺序与冲突处理。
- 代际管理：构建、切换、回滚、列表。
- 最小主机配置覆盖：用户、服务、环境、包。

退出标准
- 最小系统配置可构建、可激活、可回滚。
- 模块选项类型在编译期拦截错误。

## 阶段 D：生态与超越

目标：完整的分发链路与可贡献生态。

工作项
- 二进制缓存与替换器协议（基于 NAR）。
- 注册表与包元数据标准。
- 标准库扩展与稳定性保证。
- 文档、教程与贡献流程固化。

退出标准
- 可验证替换器支撑的可复现二进制分发。
- 贡献者流程文档化并自动化。

## 语法变更政策

所有语法变更必须：
- 先更新 `docs/spec.md`。
- 通过解析歧义与语法金样测试。
- 同步更新格式化器、LSP 语义 token、诊断。
- 避免引入增加歧义的“兼容性补丁”。

