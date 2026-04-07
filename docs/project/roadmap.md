<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Roadmap</h1>

<p><em>路线图</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

---

This roadmap turns the "inherit and surpass" vision into a phased execution plan.
Neve is a from-scratch language. Syntax decisions are not constrained by Nix compatibility.

这份路线图把“继承并超越”的愿景落实为分阶段的执行计划。
Neve 是一门从头设计的语言，语法决策不受 Nix 兼容性约束。

For the focused roadmap on making Neve a complete standalone language and a system-level scripting language,
see [language-roadmap.md](language-roadmap.md).

如果你关心的是“把 Neve 做成独立完备语言”和“做成系统级脚本语言”的专项路线图，
请直接看 [language-roadmap.md](language-roadmap.md)。

## Guiding Constraints / 指导约束

- From-scratch syntax: no compatibility targets with nixpkgs or Nix expressions. / 从头设计语法：不以兼容 nixpkgs 或 Nix 表达式为目标。
- Zero ambiguity: every syntax form must parse to exactly one meaning. / 零歧义：每一种语法形式只允许一种解析结果。
- Syntax unity: similar concepts use similar syntax and semantics. / 语法统一：相似概念使用相似的语法与语义。
- Explicit delimiters: no indentation-sensitive parsing. / 显式分隔：不使用缩进敏感解析。
- Pure functional core: effects must be explicit and isolated from evaluation. / 纯函数核心：副作用必须显式隔离，不能混入求值过程。

## Phase A: Core Language Stability / 阶段 A：语言核心稳定

Goal: finish the in-progress core and freeze the language surface for v1.
目标：完成进行中的核心能力，并冻结 v1 语言表面。

Work items / 工作项
- Complete module loader (content-addressed cache and invalidation rules). / 完成模块加载器（内容寻址缓存与失效规则）。
- Associated types in the trait system. / Trait 系统的关联类型。
- Tail-call optimization correctness and limits. / 尾调用优化的正确性与边界。
- LSP essentials: diagnostics, hover, go-to-def, basic completion. / LSP 基础能力：诊断、悬停、跳转、基础补全。
- Error reporting polish and formatter stability. / 错误信息与格式化器稳定。

Exit criteria / 退出标准
- Spec v2.0 frozen and referenced by parser/typeck/eval tests. / 规范 v2.0 冻结，并由 parser/typeck/eval 测试覆盖。
- Parser ambiguity tests and syntax change checklist in place. / 解析歧义测试与语法变更清单到位。
- LSP baseline experience: parse + type diagnostics within a single file. / LSP 基线体验：单文件内可给出解析与类型诊断。
- Formatter is idempotent across the test corpus. / 格式化器对测试语料幂等。

## Phase B: Package Manager MVP / 阶段 B：包管理 MVP

Goal: a minimal, reliable build graph with reproducible outputs and locking.
目标：最小可用、可靠的构建图，具备可复现输出与锁定。

Work items / 工作项
- Dependency resolver with deterministic lockfile format. / 依赖解析器与确定性的 lockfile 格式。
- Fetch cache and verification pipeline (URL, git, local). / Fetch 缓存与校验流水线（URL、git、本地）。
- Store integration with GC and content-addressed validation. / Store 与 GC、内容寻址校验的整合。
- Builder lifecycle: inputs -> sandbox -> outputs -> store registration. / 构建生命周期：输入 -> 沙箱 -> 输出 -> Store 注册。
- CLI flows: build/search/update basics. / CLI 基本流程：build/search/update。

Exit criteria / 退出标准
- Build a package twice and get identical store paths. / 同一包构建两次得到相同的 store 路径。
- Lockfile ensures repeatable resolution across machines. / lockfile 能跨机器复现解析结果。
- GC can free unreachable paths safely. / GC 能安全清理不可达路径。

## Phase C: Configuration System MVP / 阶段 C：配置系统 MVP

Goal: typed, composable system configuration with generations and rollback.
目标：带类型的可组合系统配置，支持代际与回滚。

Work items / 工作项
- Strongly typed options for modules and configuration merge semantics. / 模块选项强类型与配置合并语义。
- Module imports with deterministic ordering and conflict resolution. / 模块导入的确定性顺序与冲突处理。
- Generation management: build, switch, rollback, list. / 代际管理：构建、切换、回滚、列表。
- Minimal host config coverage: users, services, environment, packages. / 最小主机配置覆盖：用户、服务、环境、包。

Exit criteria / 退出标准
- A minimal system config can be built, activated, and rolled back. / 最小系统配置可构建、可激活、可回滚。
- Module option typing catches errors at compile time. / 模块选项类型在编译期拦截错误。

## Phase D: Ecosystem and Beyond / 阶段 D：生态与超越

Goal: a complete distribution pipeline and contributor-friendly ecosystem.
目标：完整的分发链路与可贡献生态。

Work items / 工作项
- Binary cache and substituter protocol (NAR-based). / 二进制缓存与替换器协议（基于 NAR）。
- Registry and package metadata standard. / 注册表与包元数据标准。
- Standard library growth with stability guarantees. / 标准库扩展与稳定性保证。
- Documentation, tutorials, and contribution workflows hardened. / 文档、教程与贡献流程固化。

Exit criteria / 退出标准
- Reproducible binary distribution with verified substitutes. / 可验证替换器支撑的可复现二进制分发。
- Contributor workflow documented and automated. / 贡献者流程文档化并自动化。

## Syntax Change Policy / 语法变更政策

All syntax changes must:

所有语法变更必须：

- Update `docs/reference/spec.md` first. / 先更新 `docs/reference/spec.md`。
- Pass parser ambiguity and golden syntax tests. / 通过解析歧义与语法金样测试。
- Update formatter, LSP tokens, and diagnostics together. / 同步更新格式化器、LSP 语义 token、诊断。
- Avoid "compatibility" hacks that increase ambiguity. / 避免引入增加歧义的“兼容性补丁”。
