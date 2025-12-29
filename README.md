# Neve

> A pure functional language for system configuration and package management.
>
> 一门用于系统配置与包管理的纯函数式语言。

---

## English

Neve inherits the core ideas from Nix (pure functional, reproducible, declarative) while building a completely new technology stack from scratch. It's not a Nix replacement or compatibility layer - it's a clean-slate reimplementation with modern language design.

### Current Status

**Language Core**: 95% complete - Full lexer, parser, type checker, and evaluator
**Toolchain**: 80% complete - LSP, formatter, REPL all working
**Package Management**: 60% complete - Derivations, store, builder implemented
**OS Integration**: 40% complete - Config framework in place

### Features

#### Implemented ✅
- **Lexer & Parser** - Complete Neve syntax parsing with error recovery
- **Type Checker** - Full Hindley-Milner type inference with trait support
- **Evaluator** - Tree-walking interpreter with lazy evaluation support
- **LSP** - Editor support with semantic highlighting and symbol indexing
- **Formatter** - Code formatting with configurable style
- **REPL** - Interactive evaluation environment
- **Store** - Content-addressed storage system
- **Derivations** - Package build model with hash verification
- **Fetcher** - Source fetching from URLs, Git repos, and local paths
- **Builder** - Sandboxed build execution (Linux namespaces)
- **Config** - System configuration with generations and activation
- **Standard Library** - Built-in modules: io, list, map, math, option, path, result, set, string

#### In Progress 🔄
- Module system refinement (visibility, re-exports)
- Trait system enhancements (associated types fully working)
- Macro system design
- Binary cache infrastructure

### A Taste of Neve

```neve
-- Define a simple package
let hello = derivation #{
    name = "hello",
    version = "2.12",
    src = fetchurl #{
        url = "https://ftp.gnu.org/gnu/hello/hello-2.12.tar.gz",
        sha256 = "cf04af86dc085268c5f4470fbae49b18...",
    },
    build = fn(src) #{
        configure = "./configure --prefix=$out",
        make = "make install",
    },
};

-- System configuration
let mySystem = #{
    hostname = "wonderland",
    users = [
        #{ name = "alice", shell = "/bin/zsh" },
    ],
    packages = [hello, git, vim],
};
```

### Syntax Highlights

| Feature | Neve Syntax | Benefit |
|---------|-------------|---------|
| Records | `#{ x = 1 }` | Unambiguous, never confused with code blocks |
| Lambda | `fn(x) x + 1` | Clear, consistent with named functions |
| Lists | `[1, 2, 3]` | Comma-separated, no confusion |
| Interpolation | `` `hello {name}` `` | Distinct from shell syntax |
| Comments | `-- comment --` | Symmetric, supports multiline |
| Pipe | `x \|> f \|> g` | Data flow clarity |
| Safe access | `x?.field` | Optional chaining |
| Error propagation | `expr?` | Result/Option unwrapping |

### Why Neve?

I love Nix's ideas but wanted to take them further with modern language design:

| Pain Point | Nix | Neve |
|------------|-----|------|
| Is this a record or function? | `{ x = 1; }` | `#{ x = 1 }` (always a record) |
| Lambda syntax conflicts with types | `x: x + 1` | `fn(x) x + 1` |
| Implicit recursion | `rec { }` | Automatic detection |
| No type safety | Runtime errors | Catch errors early |
| Inherit syntax | `inherit x;` | `#{ x }` shorthand |

### Platform Support

Neve runs on all major platforms with varying feature availability:

| Feature | Linux | macOS | Windows |
|---------|-------|-------|---------|
| Language Core (eval, check) | ✅ | ✅ | ✅ |
| REPL | ✅ | ✅ | ✅ |
| Formatter | ✅ | ✅ | ✅ |
| LSP | ✅ | ✅ | ✅ |
| Native Sandbox Build | ✅ | ❌ | ❌ |
| Docker Build | ✅ | ✅ | ✅ |
| System Configuration | ✅ | ❌ | ❌ |

Use `neve info --platform` to check your platform's capabilities.

### Installation

#### Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/MCB-SMART-BOY/neve/releases):

- **Linux (x86_64)**: `neve-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `neve-aarch64-unknown-linux-gnu.tar.gz`
- **macOS (Intel)**: `neve-x86_64-apple-darwin.tar.gz`
- **macOS (Apple Silicon)**: `neve-aarch64-apple-darwin.tar.gz`
- **Windows (x86_64)**: `neve-x86_64-pc-windows-msvc.zip`

```bash
# Linux/macOS
tar xzf neve-*.tar.gz
sudo mv neve /usr/local/bin/

# Windows: Extract and add to PATH
```

#### Building from Source

```bash
git clone https://github.com/mcbgaruda/neve.git
cd neve
cargo build --release
```

#### Arch Linux (AUR)

```bash
yay -S neve-git
```

### CLI Usage

```bash
# Basic operations
neve eval "1 + 2"              # Evaluate an expression
neve run file.neve             # Run a Neve file
neve check file.neve           # Type check a file
neve repl                      # Start interactive REPL

# Formatting
neve fmt file file.neve        # Format a file
neve fmt check file.neve       # Check formatting
neve fmt dir ./src             # Format a directory

# Package management
neve build                     # Build a package
neve package install <pkg>     # Install a package
neve package remove <pkg>      # Remove a package
neve package list              # List installed packages
neve search <query>            # Search for packages
neve info <pkg>                # Show package info

# System configuration
neve config build              # Build system configuration
neve config switch             # Switch to new configuration
neve config rollback           # Rollback to previous
neve config list               # List generations

# Store management
neve store gc                  # Run garbage collection
neve store info                # Show store information
```

### Project Structure

```
neve/
├── crates/
│   ├── neve-common      # Shared utilities (interner, spans)
│   ├── neve-diagnostic  # Error reporting with codes
│   ├── neve-lexer       # Tokenizer (logos-based)
│   ├── neve-syntax      # AST definitions
│   ├── neve-parser      # Recursive descent parser (LL(1))
│   ├── neve-hir         # HIR and name resolution
│   ├── neve-typeck      # Type inference + trait resolution
│   ├── neve-eval        # Tree-walking interpreter
│   ├── neve-std         # Standard library (9 modules)
│   ├── neve-derive      # Derivation model
│   ├── neve-store       # Content-addressed store
│   ├── neve-fetch       # Source fetching (URL, Git, local)
│   ├── neve-builder     # Sandboxed builder (Linux)
│   ├── neve-config      # System configuration + generations
│   ├── neve-fmt         # Code formatter
│   └── neve-lsp         # Language server
├── neve-cli/            # Command line interface
└── tests/               # Integration tests
```

### Type System

Neve uses Hindley-Milner type inference:

```neve
-- Types are inferred
let x = 42;                    -- x: Int
let f = fn(x) x + 1;           -- f: Int -> Int
let xs = [1, 2, 3];            -- xs: List<Int>

-- Or explicitly annotated
let y: Float = 3.14;
fn add(a: Int, b: Int) -> Int = a + b;

-- Generics
fn identity<T>(x: T) -> T = x;

-- Traits
trait Show {
    fn show(self) -> String;
};

impl Show for Int {
    fn show(self) -> String = `{self}`;
};
```

### Contributing

Contributions are welcome! If you:

- Find bugs
- Have ideas for better syntax
- Want to help implement features
- Just want to chat about language design

Please open an issue or PR!

### Name

*Neve* means "snow" in Italian and Portuguese - a nod to Nix (Latin for "snow"), but representing a fresh start.

### License

[MPL-2.0](LICENSE)

---

## 中文

Neve 继承了 Nix 的核心理念（纯函数式、可复现、声明式），同时从零构建全新的技术栈。它不是 Nix 的替代品或兼容层，而是用现代语言设计重新实现的独立生态系统。

### 当前状态

**语言核心**：95% 完成 - 完整的词法分析器、语法分析器、类型检查器和求值器
**工具链**：80% 完成 - LSP、格式化器、REPL 都已可用
**包管理**：60% 完成 - Derivations、Store、Builder 已实现
**操作系统集成**：40% 完成 - 配置框架已就位

### 功能特性

#### 已实现 ✅
- **词法分析 & 语法分析** - 完整的 Neve 语法解析，支持错误恢复
- **类型检查** - 完整的 Hindley-Milner 类型推导，支持 Trait
- **求值器** - 树遍历解释器，支持惰性求值
- **LSP** - 编辑器支持，包含语义高亮和符号索引
- **格式化器** - 可配置风格的代码格式化
- **REPL** - 交互式求值环境
- **Store** - 内容寻址存储系统
- **Derivations** - 带哈希验证的包构建模型
- **Fetcher** - 从 URL、Git 仓库、本地路径获取源码
- **Builder** - 沙箱构建执行（Linux 命名空间）
- **Config** - 系统配置，支持代际管理和激活
- **标准库** - 内置模块：io、list、map、math、option、path、result、set、string

#### 进行中 🔄
- 模块系统完善（可见性、重导出）
- Trait 系统增强（关联类型完善）
- 宏系统设计
- 二进制缓存基础设施

### Neve 长什么样

```neve
-- 定义一个简单的包
let hello = derivation #{
    name = "hello",
    version = "2.12",
    src = fetchurl #{
        url = "https://ftp.gnu.org/gnu/hello/hello-2.12.tar.gz",
        sha256 = "cf04af86dc085268c5f4470fbae49b18...",
    },
    build = fn(src) #{
        configure = "./configure --prefix=$out",
        make = "make install",
    },
};

-- 系统配置
let mySystem = #{
    hostname = "wonderland",
    users = [
        #{ name = "alice", shell = "/bin/zsh" },
    ],
    packages = [hello, git, vim],
};
```

### 语法亮点

| 特性 | Neve 语法 | 优势 |
|------|-----------|------|
| 记录 | `#{ x = 1 }` | 无歧义，不与代码块混淆 |
| Lambda | `fn(x) x + 1` | 清晰，与命名函数一致 |
| 列表 | `[1, 2, 3]` | 逗号分隔，无歧义 |
| 插值 | `` `hello {name}` `` | 与 Shell 语法区分 |
| 注释 | `-- 注释 --` | 对称，支持多行 |
| 管道 | `x \|> f \|> g` | 数据流清晰 |
| 安全访问 | `x?.field` | 可选链 |
| 错误传播 | `expr?` | Result/Option 解包 |

### 为什么选择 Neve？

我热爱 Nix 的理念，但想用现代语言设计将其推向更远：

| 痛点 | Nix | Neve |
|------|-----|------|
| 这是记录还是函数？ | `{ x = 1; }` | `#{ x = 1 }` (永远是记录) |
| Lambda 语法和类型冲突 | `x: x + 1` | `fn(x) x + 1` |
| 隐式递归 | `rec { }` | 自动检测 |
| 没有类型安全 | 运行时报错 | 提前发现错误 |
| Inherit 语法 | `inherit x;` | `#{ x }` 简写 |

### 平台支持

Neve 在所有主要平台上运行，功能支持如下：

| 功能 | Linux | macOS | Windows |
|------|-------|-------|---------|
| 语言核心 (eval, check) | ✅ | ✅ | ✅ |
| REPL | ✅ | ✅ | ✅ |
| 格式化器 | ✅ | ✅ | ✅ |
| LSP | ✅ | ✅ | ✅ |
| 原生沙箱构建 | ✅ | ❌ | ❌ |
| Docker 构建 | ✅ | ✅ | ✅ |
| 系统配置 | ✅ | ❌ | ❌ |

使用 `neve info --platform` 查看你的平台能力。

### 安装

#### 预编译二进制

从 [GitHub Releases](https://github.com/MCB-SMART-BOY/neve/releases) 下载适合你平台的版本：

- **Linux (x86_64)**: `neve-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `neve-aarch64-unknown-linux-gnu.tar.gz`
- **macOS (Intel)**: `neve-x86_64-apple-darwin.tar.gz`
- **macOS (Apple Silicon)**: `neve-aarch64-apple-darwin.tar.gz`
- **Windows (x86_64)**: `neve-x86_64-pc-windows-msvc.zip`

```bash
# Linux/macOS
tar xzf neve-*.tar.gz
sudo mv neve /usr/local/bin/

# Windows: 解压并添加到 PATH
```

#### 从源码构建

```bash
git clone https://github.com/mcbgaruda/neve.git
cd neve
cargo build --release
```

#### Arch Linux (AUR)

```bash
yay -S neve-git
```

### CLI 使用

```bash
# 基本操作
neve eval "1 + 2"              # 求值表达式
neve run file.neve             # 运行 Neve 文件
neve check file.neve           # 类型检查文件
neve repl                      # 启动交互式 REPL

# 格式化
neve fmt file file.neve        # 格式化文件
neve fmt check file.neve       # 检查格式化
neve fmt dir ./src             # 格式化目录

# 包管理
neve build                     # 构建包
neve package install <pkg>     # 安装包
neve package remove <pkg>      # 移除包
neve package list              # 列出已安装包
neve search <query>            # 搜索包
neve info <pkg>                # 显示包信息

# 系统配置
neve config build              # 构建系统配置
neve config switch             # 切换到新配置
neve config rollback           # 回滚到上一配置
neve config list               # 列出代际

# Store 管理
neve store gc                  # 运行垃圾回收
neve store info                # 显示 store 信息
```

### 项目结构

```
neve/
├── crates/
│   ├── neve-common      # 共享工具 (字符串池, 位置信息)
│   ├── neve-diagnostic  # 错误报告（含错误码）
│   ├── neve-lexer       # 词法分析（基于 logos）
│   ├── neve-syntax      # AST 定义
│   ├── neve-parser      # 递归下降解析器 (LL(1))
│   ├── neve-hir         # HIR 和名称解析
│   ├── neve-typeck      # 类型推导 + Trait 解析
│   ├── neve-eval        # 树遍历解释器
│   ├── neve-std         # 标准库 (9 个模块)
│   ├── neve-derive      # 推导模型
│   ├── neve-store       # 内容寻址存储
│   ├── neve-fetch       # 源码获取 (URL, Git, 本地)
│   ├── neve-builder     # 沙箱构建器 (Linux)
│   ├── neve-config      # 系统配置 + 代际管理
│   ├── neve-fmt         # 代码格式化
│   └── neve-lsp         # 语言服务器
├── neve-cli/            # 命令行界面
└── tests/               # 集成测试
```

### 类型系统

Neve 使用 Hindley-Milner 类型推导：

```neve
-- 类型自动推导
let x = 42;                    -- x: Int
let f = fn(x) x + 1;           -- f: Int -> Int
let xs = [1, 2, 3];            -- xs: List<Int>

-- 或显式注解
let y: Float = 3.14;
fn add(a: Int, b: Int) -> Int = a + b;

-- 泛型
fn identity<T>(x: T) -> T = x;

-- Trait
trait Show {
    fn show(self) -> String;
};

impl Show for Int {
    fn show(self) -> String = `{self}`;
};
```

### 参与贡献

欢迎贡献！如果你：

- 发现了 bug
- 对语法设计有更好的想法
- 想帮忙实现某些功能
- 只是想聊聊语言设计

欢迎开 issue 或 PR！

### 名字的由来

*Neve* 在意大利语和葡萄牙语中意为"雪"——呼应 Nix（拉丁语的"雪"），但代表着一个全新的开始。

### 许可证

[MPL-2.0](LICENSE)
