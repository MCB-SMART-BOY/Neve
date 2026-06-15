<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Documentation Hub</h1>

<p><em>文档中心</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong> ·
  <strong><a href="user/quickstart.md">Quickstart</a></strong> ·
  <strong><a href="reference/spec.md">Spec</a></strong> ·
  <strong><a href="reference/api.md">API</a></strong>
</p>

</div>

**当前状态**: Phase 6 (Syntax Overhaul v3.0) 已完成 ✅ — v3.18.0, E2E 450 测试, Formatter 幂等性 37/37, Stream<T> 14 APIs ✅, LSP 20 methods ✅。Phase 5 (生态补完) 进行中 🔄 (~95% 完成)。详见 [稳定性分级](reference/stability.md) · [生态设计](project/ecosystem-design.md) · [Stream<T> 设计](project/stream-design.md)。

Welcome to the Neve documentation hub. For **developer reference** (per-crate APIs,
architecture, integration points), see `.claude/skills/` — one skill per crate,
kept in sync with the actual code.

欢迎来到 Neve 文档中心。**开发者参考**（per-crate API、架构、集成点）请见
`.claude/skills/` — 每个 crate 一个 skill，与实际代码保持同步。

---

## Start Here / 从这里开始

- New user / 初次上手: [user/install.md](user/install.md), [user/quickstart.md](user/quickstart.md), [user/tutorial.md](user/tutorial.md)
- Language reference / 语言参考: [reference/spec.md](reference/spec.md), [reference/api.md](reference/api.md), [reference/diagnostics.md](reference/diagnostics.md)
- Contributor / 贡献者: [contributor/contributing.md](contributor/contributing.md), [contributor/onboarding.md](contributor/onboarding.md), [contributor/architecture.md](contributor/architecture.md)
- Project status / 项目现状: [project/feature-matrix.md](project/feature-matrix.md), [project/language-roadmap.md](project/language-roadmap.md), [project/changelog.md](project/changelog.md)
- Stability / 稳定性: [reference/stability.md](reference/stability.md)
- **Developer skills** / 开发者技能: [`.claude/skills/`](../.claude/skills/) — per-crate architecture, APIs, key files

---

## By Audience / 按读者角色

### I want to use Neve / 我想直接开始用

- [user/install.md](user/install.md): installation, platform notes, binary cache setup
- [user/quickstart.md](user/quickstart.md): quickest path to first expression and file
- [user/tutorial.md](user/tutorial.md): learn the language surface systematically

### I want exact language truth / 我想看严格定义

- [reference/spec.md](reference/spec.md): syntax and semantic rules
- [reference/api.md](reference/api.md): standard library reference
- [reference/diagnostics.md](reference/diagnostics.md): diagnostic code index

### I want to understand project reality / 我想看项目真实完成度

- [project/feature-matrix.md](project/feature-matrix.md): real support matrix
- [project/bash-nix-roadmap.md](project/bash-nix-roadmap.md): Bash/Nix replacement roadmap
- [project/language-roadmap.md](project/language-roadmap.md): language completion roadmap
- [project/changelog.md](project/changelog.md): released changes only

### I want to contribute / 我想参与开发

- [contributor/contributing.md](contributor/contributing.md): setup, workflow, style
- [contributor/onboarding.md](contributor/onboarding.md): codebase reading order
- [contributor/architecture.md](contributor/architecture.md): crate responsibilities and pipeline
- [contributor/bootstrap.md](contributor/bootstrap.md): bootstrap package examples now kept under `examples/bootstrap/`

---

## CLI Docs / CLI 文档入口

```neve
neve doc index
neve doc quickstart
neve doc spec
neve doc api
neve doc contributing
neve doc feature-matrix
```

For command behavior, use `neve --help`.
命令行为请直接看 `neve --help`。

---

<div align="center">

**[Main README](../README.md)** · **[License: MPL-2.0](../LICENSE)**

</div>
