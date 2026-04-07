<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p><em>A standalone language for system configuration and structured shell automation / 面向系统配置与结构化 shell 自动化的独立语言</em></p>

<p>
  <a href="https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml">
    <img src="https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/MCB-SMART-BOY/neve/releases">
    <img src="https://img.shields.io/github/v/release/MCB-SMART-BOY/neve?include_prereleases&color=blue" alt="Release">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg" alt="License">
  </a>
  <a href="https://aur.archlinux.org/packages/neve-bin">
    <img src="https://img.shields.io/aur/version/neve-bin?color=1793d1&label=AUR" alt="AUR">
  </a>
</p>

<p><strong>Windows</strong> · <strong>Linux</strong> · <strong>macOS</strong></p>

</div>
> *Nix's design ambition. Rust's safety mindset. A standalone language for system configuration and structured shell automation.*  
> *Nix 的设计野心，Rust 的安全取向，面向系统配置与结构化 shell 自动化的独立语言。*

Neve is a standalone programming language for system configuration, package workflows, and eventually safer replacement of ad-hoc Bash-style automation.
Neve 不是 Nix 换皮，也不是 shell 宏层。它是一门独立编程语言，面向系统配置、包工作流，并以类型化、结构化的方式逐步替代随意堆砌的 Bash 自动化。

**Quick Links / 快速链接** · [Docs](docs/) · [Install](docs/user/install.md) · [Quickstart](docs/user/quickstart.md) · [Feature Matrix](docs/project/feature-matrix.md) · [Issues](https://github.com/MCB-SMART-BOY/neve/issues)

### What Neve Is / Neve 是什么

- New-era language with a Rust-style safety mindset / 新时代、强调安全性与诊断质量的语言
- Nix-inspired design philosophy without Nix syntax baggage / 继承 Nix 哲学，但不背负其历史语法包袱
- Standalone programming language, not a config-only DSL / 独立编程语言，而不只是配置 DSL
- System-level configuration, package/build orchestration, and reproducible workflows / 面向系统配置、包管理、构建编排与可复现工作流
- Aiming to cover traditional shell workflows with structured, typed APIs / 目标是用结构化、类型化 API 覆盖传统 shell 工作流

### Why It Exists / 为什么做 Neve

Neve tries to combine four things that are usually split apart: Nix-style declarative system thinking, Rust-style safety expectations, a real general-purpose language surface, and shell-grade automation needs.
Neve 想把四类通常被拆开的能力放进同一门语言里：Nix 式声明式系统思维、Rust 式安全预期、真正可编程的语言表面，以及能承载 shell 级自动化的执行能力。

| Pain Point / 痛点 | Legacy tool / 传统方案 | Neve |
|:------------------|:-----------------------|:-----|
| Config vs programming / 配置与编程割裂 | Config DSL + another language | One language surface / 同一语言表面 |
| Syntax ambiguity / 语法歧义 | Record/function forms often collide | `#{ ... }` vs `fn(...)` |
| Errors arrive too late / 错误来得太晚 | Runtime explosion / 运行时爆炸 | Static checking first / 先做静态检查 |
| Shell automation gets fragile / shell 自动化脆弱 | Strings, exit codes, hidden state | Typed, structured workflows as the target / 目标是结构化、可类型化 |

### 30-Second Demo / 30 秒感受一下

```neve
let service = #{
    name = "web",
    port = 8080,
};

fn unit_file(s: #{ name: String, port: Int }) -> String = `
[Service]
Environment=PORT={s.port}
ExecStart=/usr/bin/{s.name} --port {s.port}
`;

unit_file(service)
```

### Start Here / 从这里开始

- [Install](docs/user/install.md): installation methods, package managers, platform notes
- [Quickstart](docs/user/quickstart.md): first expression, first file, first REPL session
- [Tutorial](docs/user/tutorial.md): language walkthrough from basics to modules
- [Spec](docs/reference/spec.md): exact language rules
- [API](docs/reference/api.md): standard library reference
- [Feature Matrix](docs/project/feature-matrix.md): real support status, not marketing status

### Install / 安装

```bash
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh

# Cargo
cargo install neve
```

Windows, Homebrew, AUR, Nix, source builds, binary cache, and platform notes:
更完整的安装方式、包管理器入口、二进制缓存与平台说明：
[docs/user/install.md](docs/user/install.md)

### Current Reality / 当前现状

- Core language, parser, type checker, formatter, and CLI are usable.
- HIR canonical pipeline and tooling are still converging.
- LSP, package workflows, and system configuration are still being hardened.
- Full Bash/shell replacement is a roadmap goal, not current shipped reality.

真实支持范围请直接看：
- [Feature Matrix](docs/project/feature-matrix.md)
- [Roadmap](docs/project/roadmap.md)
- [Language Roadmap](docs/project/language-roadmap.md)

### Contributing / 贡献

Contributions are welcome. Start here:
欢迎贡献，从这里开始：

- [Contributor Guide](docs/contributor/contributing.md)
- [Architecture](docs/contributor/architecture.md)
- [Onboarding](docs/contributor/onboarding.md)
- [Brand Assets](assets/README.md)

### License / 许可证

Neve is licensed under the [Mozilla Public License 2.0](LICENSE).
Neve 使用 [Mozilla Public License 2.0](LICENSE) 许可。
