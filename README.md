<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p><em>A pure functional language for system configuration / 面向系统配置的纯函数式语言</em></p>

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
> *Nix's soul. Better syntax. Type safety.*  
> *Nix 的灵魂，更好的语法，类型安全。*

Neve is a pure functional programming language designed for system configuration and package management. It takes the powerful concepts from Nix—reproducibility, declarative configuration, and functional purity—while providing a cleaner, more intuitive syntax and compile-time type checking.
Neve 是一门纯函数式编程语言，专为系统配置与包管理而设计。它继承了 Nix 的强大理念，同时提供更清晰、更直观的语法与编译期类型检查。

**Quick Links / 快速链接** · [Docs](docs/) · [Changelog](docs/changelog.md) · [Issues](https://github.com/MCB-SMART-BOY/neve/issues)

### Highlights / 亮点

- Unambiguous records vs functions (`#{ ... }` vs `fn(...)`) / 记录与函数语法不歧义
- Strong static typing with helpful diagnostics / 强类型与更好的诊断提示
- Lazy evaluation where it matters / 关键路径惰性求值
- Modern ergonomics: pipelines, pattern matching, interpolation / 现代语法：管道、模式匹配、插值
- Practical focus: system config + package workflows / 面向系统配置与包管理的实践

### Why Neve? / 为什么选择 Neve？

| Pain Point / 痛点 | Nix | Neve |
|:------------------|:----|:-----|
| Is this a record or function? / 这是记录还是函数？ | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| Type errors / 类型错误 | Runtime explosion / 运行时爆炸 | Compile-time catch / 编译期捕获 |
| String interpolation / 字符串插值 | `"${x}"` | `` `{x}` `` |
| Recursion / 递归 | `rec { ... }` | Just works / 自动处理 |

### Quick Demo / 快速演示

```bash
$ neve repl
neve> #{ name = "world", greet = fn(n) `Hello, {n}!` }
#{greet = <fn>, name = "world"}
neve> let r = #{ name = "world", greet = fn(n) `Hello, {n}!` }
neve> r.greet(r.name)
"Hello, world!"
```

### Installation / 安装

#### Quick Install (Recommended) / 快速安装（推荐）

<table>
<tr>
<td width="50%">

**Linux / macOS**

```bash
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh
```

</td>
<td width="50%">

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.ps1 | iex
```

</td>
</tr>
</table>

#### Package Managers / 包管理器

<table>
<tr>
<th>Platform / 平台</th>
<th>Command / 命令</th>
<th>Notes / 说明</th>
</tr>
<tr>
<td><b>Arch Linux</b></td>
<td>

```bash
yay -S neve-bin
```

</td>
<td>Prebuilt binary, fastest install / 预编译二进制，最快安装</td>
</tr>
<tr>
<td><b>Arch Linux</b></td>
<td>

```bash
yay -S neve-git
```

</td>
<td>Build from source, latest features / 源码构建，最新特性</td>
</tr>
<tr>
<td><b>macOS</b></td>
<td>

```bash
brew tap MCB-SMART-BOY/neve
brew install neve
```

</td>
<td>Intel & Apple Silicon / Intel 与 Apple Silicon</td>
</tr>
<tr>
<td><b>Nix</b></td>
<td>

```bash
nix run github:MCB-SMART-BOY/nix-neve
```

</td>
<td>Try without installing / 免安装试用</td>
</tr>
<tr>
<td><b>Nix</b></td>
<td>

```bash
nix profile install github:MCB-SMART-BOY/nix-neve
```

</td>
<td>Install to profile / 安装到 profile</td>
</tr>
<tr>
<td><b>Cargo</b></td>
<td>

```bash
cargo install neve
```

</td>
<td>Requires Rust toolchain / 需要 Rust 工具链</td>
</tr>
</table>

#### From Source / 从源码编译

```bash
git clone https://github.com/MCB-SMART-BOY/neve
cd neve
cargo build --release
# Binary at ./target/release/neve
```

### Language Features / 语言特性

#### Records & Functions / 记录与函数

```neve
-- Records use #{ } syntax (never ambiguous with functions)
let config = #{
    port = 8080,
    host = "localhost",
    debug = true,
};

-- Functions use fn keyword
fn greet(name) = `Hello, {name}!`;

-- Multiple parameters
fn add(a, b) = a + b;
```

#### Pattern Matching / 模式匹配

```neve
fn describe(value) = match value {
    0 -> "zero",
    1 -> "one",
    n if n < 0 -> "negative",
    n -> `positive: {n}`,
};

fn factorial(n) = match n {
    0 -> 1,
    n -> n * factorial(n - 1),
};
```

#### Pipe Operator / 管道操作符

```neve
-- Chain operations naturally
let result = [1, 2, 3, 4, 5]
    |> filter(fn(x) x > 2)
    |> map(fn(x) x * 2)
    |> fold(0, fn(a, b) a + b);
```

#### Type Annotations / 类型标注

```neve
fn add(a: Int, b: Int) -> Int = a + b;

let config: #{ port: Int, host: String } = #{
    port = 8080,
    host = "localhost",
};
```

### CLI Usage / 命令行用法

```bash
neve repl              # Interactive REPL / 交互式 REPL
neve eval "1 + 2"      # Evaluate expression / 计算表达式
neve run file.neve     # Run a file / 运行文件
neve check file.neve   # Type check without running / 仅类型检查
neve fmt file.neve     # Format code / 格式化代码
neve doc               # View documentation / 查看文档
neve doc quickstart    # Quick start guide / 快速入门
neve doc spec          # Language specification / 语言规范
```

### Documentation / 文档

Built-in documentation is available via `neve doc`:
内置文档可通过 `neve doc` 查看：

| Topic / 主题 | Command / 命令 | Description / 说明 |
|:-------------|:---------------|:-------------------|
| Quick Start / 快速入门 | `neve doc quickstart` | 5-minute introduction / 5 分钟入门 |
| Specification / 语言规范 | `neve doc spec` | Complete language reference / 完整参考 |
| API Reference / 标准库 | `neve doc api` | Standard library docs / 标准库文档 |
| Diagnostics / 诊断 | `neve doc diagnostics` | Error code reference / 错误码说明 |
| Architecture / 架构 | `neve doc architecture` | Internal architecture / 内部架构 |
| Onboarding / 贡献者入门 | `neve doc onboarding` | Contributor onboarding / 贡献指南 |

### Project Status / 项目进度

| Component / 模块 | Status | Description / 说明 |
|:-----------------|:-------|:-------------------|
| Lexer & Parser / 词法与语法 | ✅ Complete | Full syntax support / 完整语法支持 |
| Type Checker / 类型检查 | ✅ Complete | Hindley-Milner + Traits / HM + Traits |
| Evaluator / 求值器 | ✅ Complete | Lazy evaluation + TCO / 惰性求值 + TCO |
| REPL / 交互式 | ✅ Complete | Interactive development / 交互开发 |
| Formatter / 格式化 | ✅ Complete | Opinionated formatting / 统一风格格式化 |
| LSP / 编辑器 | 🚧 In Progress | Editor integration / 编辑器集成 |
| Package Manager / 包管理 | 🚧 In Progress | Dependency management / 依赖管理 |
| System Config / 系统配置 | 📋 Planned | NixOS-style configuration / 类 NixOS 配置 |

### Logo Assets / Logo 资源

- Primary (with glow) / 主版（含光晕）: `assets/logo.svg`
- Transparent / no-glow / 透明无光晕: `assets/logo-plain.svg`
- Size variants (transparent) / 多尺寸透明版: `assets/logo-64.svg`, `assets/logo-128.svg`, `assets/logo-256.svg`
- PNG exports / PNG 导出: `assets/logo.png`, `assets/logo-plain.png`, `assets/logo-16.png`, `assets/logo-32.png`, `assets/logo-48.png`, `assets/logo-64.png`, `assets/logo-128.png`, `assets/logo-256.png`
- ICO / 浏览器图标: `assets/logo.ico`

### Contributing / 贡献

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md)。

```bash
# Development setup
git clone https://github.com/MCB-SMART-BOY/neve
cd neve
cargo test              # Run tests
cargo run -- repl       # Test REPL
```

### License / 许可证

Neve is licensed under the [Mozilla Public License 2.0](LICENSE).
Neve 使用 [Mozilla Public License 2.0](LICENSE) 许可。

---
