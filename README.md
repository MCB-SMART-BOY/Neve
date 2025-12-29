<div align="center">

```
    _   __
   / | / /___  _   _____
  /  |/ / _ \| | / / _ \
 / /|  /  __/| |/ /  __/
/_/ |_/\___/ |___/\___/
```

### *A pure functional language for system configuration*

[![CI](https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml/badge.svg)](https://github.com/MCB-SMART-BOY/neve/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MCB-SMART-BOY/neve?include_prereleases&color=blue)](https://github.com/MCB-SMART-BOY/neve/releases)
[![License: MPL-2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](LICENSE)
[![AUR](https://img.shields.io/aur/version/neve-bin?color=1793d1&label=AUR)](https://aur.archlinux.org/packages/neve-bin)

**Windows** · **Linux** · **macOS**

---

**[English](#english)** | **[中文](#中文)**

---

</div>

## English

> *Nix's soul. Better syntax. Type safety.*

Neve is a pure functional programming language designed for system configuration and package management. It takes the powerful concepts from Nix—reproducibility, declarative configuration, and functional purity—while providing a cleaner, more intuitive syntax and compile-time type checking.

### Why Neve?

| Pain Point | Nix | Neve |
|:-----------|:----|:-----|
| Is this a record or function? | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| Type errors | Runtime explosion | Compile-time catch |
| String interpolation | `"${x}"` | `` `{x}` `` |
| Recursion | `rec { ... }` | Just works |

### Quick Demo

```bash
$ neve repl
neve> #{ name = "world", greet = fn(n) `Hello, {n}!` }
#{greet = <fn>, name = "world"}
neve> let r = #{ name = "world", greet = fn(n) `Hello, {n}!` }
neve> r.greet(r.name)
"Hello, world!"
```

### Installation

#### Quick Install (Recommended)

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

#### Package Managers

<table>
<tr>
<th>Platform</th>
<th>Command</th>
<th>Notes</th>
</tr>
<tr>
<td><b>Arch Linux</b></td>
<td>

```bash
yay -S neve-bin
```

</td>
<td>Prebuilt binary, fastest install</td>
</tr>
<tr>
<td><b>Arch Linux</b></td>
<td>

```bash
yay -S neve-git
```

</td>
<td>Build from source, latest features</td>
</tr>
<tr>
<td><b>macOS</b></td>
<td>

```bash
brew tap MCB-SMART-BOY/neve
brew install neve
```

</td>
<td>Intel & Apple Silicon</td>
</tr>
<tr>
<td><b>Nix</b></td>
<td>

```bash
nix run github:MCB-SMART-BOY/nix-neve
```

</td>
<td>Try without installing</td>
</tr>
<tr>
<td><b>Nix</b></td>
<td>

```bash
nix profile install github:MCB-SMART-BOY/nix-neve
```

</td>
<td>Install to profile</td>
</tr>
<tr>
<td><b>Cargo</b></td>
<td>

```bash
cargo install neve
```

</td>
<td>Requires Rust toolchain</td>
</tr>
</table>

#### From Source

```bash
git clone https://github.com/MCB-SMART-BOY/neve
cd neve
cargo build --release
# Binary at ./target/release/neve
```

### Language Features

#### Records & Functions

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

#### Pattern Matching

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

#### Pipe Operator

```neve
-- Chain operations naturally
let result = [1, 2, 3, 4, 5]
    |> filter(fn(x) x > 2)
    |> map(fn(x) x * 2)
    |> fold(0, fn(a, b) a + b);
```

#### Type Annotations

```neve
fn add(a: Int, b: Int) -> Int = a + b;

let config: #{ port: Int, host: String } = #{
    port = 8080,
    host = "localhost",
};
```

### CLI Usage

```bash
neve repl              # Interactive REPL
neve eval "1 + 2"      # Evaluate expression
neve run file.neve     # Run a file
neve check file.neve   # Type check without running
neve fmt file.neve     # Format code
neve doc               # View documentation
neve doc quickstart    # Quick start guide
neve doc spec          # Language specification
```

### Documentation

Built-in documentation is available via `neve doc`:

| Topic | Command | Description |
|:------|:--------|:------------|
| Quick Start | `neve doc quickstart` | 5-minute introduction |
| Specification | `neve doc spec` | Complete language reference |
| API Reference | `neve doc api` | Standard library docs |
| Examples | `neve doc examples` | Code examples |

### Project Status

| Component | Status | Description |
|:----------|:-------|:------------|
| Lexer & Parser | ✅ Complete | Full syntax support |
| Type Checker | ✅ Complete | Hindley-Milner with extensions |
| Evaluator | ✅ Complete | Lazy evaluation |
| REPL | ✅ Complete | Interactive development |
| Formatter | ✅ Complete | Opinionated formatting |
| LSP | 🚧 In Progress | Editor integration |
| Package Manager | 🚧 In Progress | Dependency management |
| System Config | 📋 Planned | NixOS-style configuration |

### Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Development setup
git clone https://github.com/MCB-SMART-BOY/neve
cd neve
cargo test              # Run tests
cargo run -- repl       # Test REPL
```

### License

Neve is licensed under the [Mozilla Public License 2.0](LICENSE).

---

## 中文

> *Nix 的灵魂，更好的语法，类型安全。*

Neve 是一门纯函数式编程语言，专为系统配置和包管理而设计。它继承了 Nix 的强大理念——可重现性、声明式配置和函数式纯净——同时提供更清晰、更直观的语法和编译期类型检查。

### 为什么选择 Neve？

| 痛点 | Nix | Neve |
|:-----|:----|:-----|
| 这是记录还是函数？ | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| 类型错误 | 运行时爆炸 | 编译期捕获 |
| 字符串插值 | `"${x}"` | `` `{x}` `` |
| 递归 | `rec { ... }` | 自动处理 |

### 快速演示

```bash
$ neve repl
neve> #{ name = "世界", greet = fn(n) `你好，{n}！` }
#{greet = <fn>, name = "世界"}
neve> let r = #{ name = "世界", greet = fn(n) `你好，{n}！` }
neve> r.greet(r.name)
"你好，世界！"
```

### 安装

#### 快速安装（推荐）

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

#### 包管理器

<table>
<tr>
<th>平台</th>
<th>命令</th>
<th>说明</th>
</tr>
<tr>
<td><b>Arch Linux</b></td>
<td>

```bash
yay -S neve-bin
```

</td>
<td>预编译二进制，安装最快</td>
</tr>
<tr>
<td><b>Arch Linux</b></td>
<td>

```bash
yay -S neve-git
```

</td>
<td>从源码编译，最新功能</td>
</tr>
<tr>
<td><b>macOS</b></td>
<td>

```bash
brew tap MCB-SMART-BOY/neve
brew install neve
```

</td>
<td>支持 Intel 和 Apple Silicon</td>
</tr>
<tr>
<td><b>Nix</b></td>
<td>

```bash
nix run github:MCB-SMART-BOY/nix-neve
```

</td>
<td>试用（不安装）</td>
</tr>
<tr>
<td><b>Nix</b></td>
<td>

```bash
nix profile install github:MCB-SMART-BOY/nix-neve
```

</td>
<td>安装到 profile</td>
</tr>
<tr>
<td><b>Cargo</b></td>
<td>

```bash
cargo install neve
```

</td>
<td>需要 Rust 工具链</td>
</tr>
</table>

#### 从源码编译

```bash
git clone https://github.com/MCB-SMART-BOY/neve
cd neve
cargo build --release
# 二进制位于 ./target/release/neve
```

### 语言特性

#### 记录与函数

```neve
-- 记录使用 #{ } 语法（与函数永不混淆）
let config = #{
    port = 8080,
    host = "localhost",
    debug = true,
};

-- 函数使用 fn 关键字
fn greet(name) = `你好，{name}！`;

-- 多参数函数
fn add(a, b) = a + b;
```

#### 模式匹配

```neve
fn describe(value) = match value {
    0 -> "零",
    1 -> "一",
    n if n < 0 -> "负数",
    n -> `正数：{n}`,
};

fn factorial(n) = match n {
    0 -> 1,
    n -> n * factorial(n - 1),
};
```

#### 管道操作符

```neve
-- 自然地链式操作
let result = [1, 2, 3, 4, 5]
    |> filter(fn(x) x > 2)
    |> map(fn(x) x * 2)
    |> fold(0, fn(a, b) a + b);
```

#### 类型标注

```neve
fn add(a: Int, b: Int) -> Int = a + b;

let config: #{ port: Int, host: String } = #{
    port = 8080,
    host = "localhost",
};
```

### 命令行用法

```bash
neve repl              # 交互式 REPL
neve eval "1 + 2"      # 求值表达式
neve run file.neve     # 运行文件
neve check file.neve   # 类型检查（不运行）
neve fmt file.neve     # 格式化代码
neve doc               # 查看文档
neve doc quickstart    # 快速入门
neve doc spec --zh     # 语言规范（中文）
```

### 文档

通过 `neve doc` 访问内置文档：

| 主题 | 命令 | 描述 |
|:-----|:-----|:-----|
| 快速入门 | `neve doc quickstart` | 5 分钟入门教程 |
| 语言规范 | `neve doc spec --zh` | 完整语言参考 |
| API 参考 | `neve doc api --zh` | 标准库文档 |
| 示例 | `neve doc examples` | 代码示例 |

### 项目进度

| 组件 | 状态 | 说明 |
|:-----|:-----|:-----|
| 词法分析器 & 语法分析器 | ✅ 完成 | 完整语法支持 |
| 类型检查器 | ✅ 完成 | 带扩展的 Hindley-Milner |
| 求值器 | ✅ 完成 | 惰性求值 |
| REPL | ✅ 完成 | 交互式开发 |
| 格式化器 | ✅ 完成 | 统一风格格式化 |
| LSP | 🚧 进行中 | 编辑器集成 |
| 包管理器 | 🚧 进行中 | 依赖管理 |
| 系统配置 | 📋 计划中 | NixOS 风格配置 |

### 贡献

欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

```bash
# 开发环境设置
git clone https://github.com/MCB-SMART-BOY/neve
cd neve
cargo test              # 运行测试
cargo run -- repl       # 测试 REPL
```

### 许可证

Neve 使用 [Mozilla Public License 2.0](LICENSE) 授权。

---

<div align="center">

**[文档](docs/)** · **[问题反馈](https://github.com/MCB-SMART-BOY/neve/issues)** · **[许可证: MPL-2.0](LICENSE)**

</div>
