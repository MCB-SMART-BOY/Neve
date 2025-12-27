# Neve

> A pure functional language for system configuration and package management.
>
> 一门用于系统配置与包管理的纯函数式语言。

---

## English

Neve is a modern replacement for the Nix language. While Nix is incredibly powerful, Neve aims to be more approachable with cleaner syntax and a proper type system.

### Features

- **Lexer & Parser** - Complete Neve syntax parsing with error recovery
- **Type Checker** - Full Hindley-Milner type inference
- **Evaluator** - Tree-walking interpreter for expressions
- **LSP** - Editor support with semantic highlighting and symbol indexing
- **Formatter** - Code formatting with configurable style
- **REPL** - Interactive evaluation environment
- **Store** - Content-addressed storage system
- **Derivations** - Package build model with hash verification
- **Fetcher** - Source fetching from URLs, Git repos, and local paths
- **Builder** - Sandboxed build execution (Linux)
- **Config** - System configuration with generations and activation
- **Standard Library** - Built-in modules for io, list, map, math, option, path, result, set, string

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

### Why Another Nix?

I love Nix's ideas but struggle with its syntax:

| Pain Point | Nix | Neve |
|------------|-----|------|
| Is this a record or function? | `{ x = 1; }` | `#{ x = 1 }` (always a record) |
| Lambda syntax conflicts with types | `x: x + 1` | `fn(x) x + 1` |
| Implicit recursion | `rec { }` | Automatic detection |
| No type safety | Runtime errors | Catch errors early |

### Installation

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
neve eval "1 + 2"              # Evaluate an expression
neve run file.neve             # Run a Neve file
neve check file.neve           # Type check a file
neve fmt file file.neve        # Format a file
neve repl                      # Start interactive REPL
neve build                     # Build a package
neve package install <pkg>     # Install a package
neve package remove <pkg>      # Remove a package
neve search <query>            # Search for packages
neve info <pkg>                # Show package info
neve config build              # Build system configuration
neve config switch             # Switch to new configuration
neve store gc                  # Run garbage collection
neve store info                # Show store information
```

### Project Structure

```
neve/
├── crates/
│   ├── neve-common      # Shared utilities (interner, spans)
│   ├── neve-diagnostic  # Error reporting
│   ├── neve-lexer       # Tokenizer
│   ├── neve-syntax      # AST definitions
│   ├── neve-parser      # Recursive descent parser
│   ├── neve-hir         # Name resolution
│   ├── neve-typeck      # Type inference
│   ├── neve-eval        # Tree-walking interpreter
│   ├── neve-std         # Standard library
│   ├── neve-derive      # Derivation model
│   ├── neve-store       # Content-addressed store
│   ├── neve-fetch       # Source fetching
│   ├── neve-builder     # Sandboxed builder
│   ├── neve-config      # System configuration
│   ├── neve-fmt         # Code formatter
│   └── neve-lsp         # Language server
├── neve-cli/            # Command line interface
└── tests/               # Integration tests
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

Neve 是 Nix 语言的现代替代品。虽然 Nix 非常强大，但 Neve 的目标是提供更清晰的语法和完善的类型系统，让它更加易用。

### 功能特性

- **词法分析 & 语法分析** - 完整的 Neve 语法解析，支持错误恢复
- **类型检查** - 完整的 Hindley-Milner 类型推导
- **求值器** - 表达式的树遍历解释器
- **LSP** - 编辑器支持，包含语义高亮和符号索引
- **格式化器** - 可配置风格的代码格式化
- **REPL** - 交互式求值环境
- **Store** - 内容寻址存储系统
- **Derivations** - 带哈希验证的包构建模型
- **Fetcher** - 从 URL、Git 仓库、本地路径获取源码
- **Builder** - 沙箱构建执行（Linux）
- **Config** - 系统配置，支持代际管理和激活
- **标准库** - 内置 io、list、map、math、option、path、result、set、string 模块

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

### 为什么要再造一个 Nix？

我喜欢 Nix 的理念，但总是被它的语法困扰：

| 痛点 | Nix | Neve |
|------|-----|------|
| 这是记录还是函数？ | `{ x = 1; }` | `#{ x = 1 }` (永远是记录) |
| Lambda 语法和类型冲突 | `x: x + 1` | `fn(x) x + 1` |
| 隐式递归 | `rec { }` | 自动检测 |
| 没有类型安全 | 运行时报错 | 提前发现错误 |

### 安装

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
neve eval "1 + 2"              # 求值表达式
neve run file.neve             # 运行 Neve 文件
neve check file.neve           # 类型检查文件
neve fmt file file.neve        # 格式化文件
neve repl                      # 启动交互式 REPL
neve build                     # 构建包
neve package install <pkg>     # 安装包
neve package remove <pkg>      # 移除包
neve search <query>            # 搜索包
neve info <pkg>                # 显示包信息
neve config build              # 构建系统配置
neve config switch             # 切换到新配置
neve store gc                  # 运行垃圾回收
neve store info                # 显示 store 信息
```

### 项目结构

```
neve/
├── crates/
│   ├── neve-common      # 共享工具 (字符串池, 位置信息)
│   ├── neve-diagnostic  # 错误报告
│   ├── neve-lexer       # 词法分析
│   ├── neve-syntax      # AST 定义
│   ├── neve-parser      # 递归下降解析器
│   ├── neve-hir         # 名称解析
│   ├── neve-typeck      # 类型推导
│   ├── neve-eval        # 树遍历解释器
│   ├── neve-std         # 标准库
│   ├── neve-derive      # 推导模型
│   ├── neve-store       # 内容寻址存储
│   ├── neve-fetch       # 源码获取
│   ├── neve-builder     # 沙箱构建器
│   ├── neve-config      # 系统配置
│   ├── neve-fmt         # 代码格式化
│   └── neve-lsp         # 语言服务器
├── neve-cli/            # 命令行界面
└── tests/               # 集成测试
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
