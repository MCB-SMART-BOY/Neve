# Neve

A pure functional language for system configuration and package management.

一门用于系统配置与包管理的纯函数式语言。

> Pure Rust | Zero Ambiguity | Unified Syntax
>
> 纯 Rust 实现 | 零二义性 | 语法统一

Neve is designed as a modern replacement for the Nix language, addressing its historical baggage while maintaining the power of declarative, reproducible system management.

Neve 旨在成为 Nix 语言的现代替代品，解决其历史包袱问题，同时保持声明式、可复现系统管理的强大能力。

## Features / 特性

- **Static Type System / 静态类型系统** - Hindley-Milner type inference with traits / 带 Trait 的 HM 类型推导
- **Zero Ambiguity / 零二义性** - Every construct has exactly one parse interpretation / 每个构造只有唯一解析方式
- **Pure Functional / 纯函数式** - No side effects, perfect reproducibility / 无副作用，完美可复现
- **Modern Syntax / 现代语法** - Clean, consistent, indentation-independent / 简洁、一致、不依赖缩进

## Quick Example / 快速示例

```neve
-- Package definition / 包定义
let hello = derivation #{
    name = "hello",
    version = "2.12",
    src = fetchurl #{
        url = "https://ftp.gnu.org/gnu/hello/hello-2.12.tar.gz",
        sha256 = "cf04af86dc085268c5f4470fbae49b18afbc221b78096aab842d934a76bad0ab",
    },
    build = fn(src) #{
        configure = "./configure --prefix=$out",
        make = "make",
        install = "make install",
    },
};

-- System configuration / 系统配置
let config = #{
    hostname = "myhost",
    users = [
        #{ name = "alice", shell = "/bin/zsh" },
        #{ name = "bob", shell = "/bin/bash" },
    ],
    packages = [hello, git, vim],
    services = #{
        sshd = #{ enable = true, port = 22 },
        nginx = #{ enable = true },
    },
};
```

## Syntax Comparison / 语法对比

| Concept / 概念 | Neve | Nix |
|----------------|------|-----|
| Records / 记录 | `#{ x = 1 }` | `{ x = 1; }` |
| Lambda / 闭包 | `fn(x) x + 1` | `x: x + 1` |
| Comments / 注释 | `-- comment --` | `# comment` |
| Interpolation / 插值 | `` `hello {name}` `` | `"hello ${name}"` |
| List concat / 列表连接 | `xs ++ ys` | `xs ++ ys` |
| Record merge / 记录合并 | `a // b` | `a // b` |
| Pipe / 管道 | `x \|> f \|> g` | N/A |
| Type annotation / 类型注解 | `x: Int` | N/A |

## Project Status / 项目状态

**Phase 1: Core Language / 核心语言** - In Progress / 进行中

| Component / 组件 | Status / 状态 |
|------------------|---------------|
| Lexer / 词法分析 | Done / 完成 |
| Parser / 语法分析 | Done / 完成 |
| HIR / Name Resolution / 名称解析 | Done / 完成 |
| Type Checker / 类型检查 | Basic / 基础 |
| Evaluator / 求值器 | Basic / 基础 |
| Standard Library / 标准库 | Skeleton / 骨架 |
| LSP / 语言服务 | Basic / 基础 |
| Formatter / 格式化 | Basic / 基础 |

**Phase 2: Package Management / 包管理** - Skeleton / 骨架

| Component / 组件 | Status / 状态 |
|------------------|---------------|
| Derivation Model / 推导模型 | Skeleton / 骨架 |
| Content-Addressed Store / 内容寻址存储 | Skeleton / 骨架 |
| Sandbox Builder / 沙箱构建器 | Skeleton / 骨架 |
| Fetchers / 获取器 | Skeleton / 骨架 |

**Phase 3-4: System Config & Tooling / 系统配置与工具** - Planned / 计划中

## Building / 构建

```bash
# Clone / 克隆
git clone https://github.com/aspect-analytics/neve.git
cd neve

# Build / 构建
cargo build --release

# Test / 测试
cargo test
```

## Architecture / 架构

```
neve/
├── crates/
│   ├── neve-common      # Span, interner, arena / 基础设施
│   ├── neve-diagnostic  # Error reporting / 错误报告
│   ├── neve-lexer       # Tokenizer / 词法分析
│   ├── neve-syntax      # AST definitions / AST 定义
│   ├── neve-parser      # Recursive descent parser / 递归下降解析器
│   ├── neve-hir         # High-level IR / 高级中间表示
│   ├── neve-typeck      # Type inference / 类型推导
│   ├── neve-eval        # Evaluator / 求值器
│   ├── neve-std         # Standard library / 标准库
│   ├── neve-derive      # Derivation model / 推导模型
│   ├── neve-store       # Content-addressed store / 内容寻址存储
│   ├── neve-fetch       # URL/Git fetchers / 获取器
│   ├── neve-builder     # Sandbox executor / 沙箱执行器
│   ├── neve-config      # System configuration / 系统配置
│   ├── neve-lsp         # Language Server / 语言服务器
│   └── neve-fmt         # Formatter / 格式化器
├── neve-cli/            # CLI application / 命令行工具
└── tests/               # Integration tests / 集成测试
```

## Design Philosophy / 设计哲学

1. **Zero Ambiguity / 零二义性** - No context-dependent parsing / 无上下文依赖解析
2. **Unified Syntax / 语法统一** - Similar concepts use similar syntax / 相似概念使用相似语法
3. **Indentation Independent / 不依赖缩进** - Explicit delimiters / 显式分隔符
4. **Pure Functional / 纯函数式** - Side effects through derivations only / 副作用仅通过推导
5. **Simplicity First / 简洁优先** - 17 keywords, minimal noise / 17 个关键字，最少噪音

See / 详见 [PHILOSOPHY.md](PHILOSOPHY.md)

## Nix vs Neve

| Nix Problem / Nix 问题 | Neve Solution / Neve 方案 |
|------------------------|---------------------------|
| `{ }` ambiguity / 二义性 | `#{ }` records, `{ }` blocks / 记录与块分离 |
| `x: x+1` lambda confusion / 闭包混淆 | `fn(x) x+1` explicit / 显式闭包 |
| `rec { }` explicit recursion / 显式递归 | Automatic detection / 自动检测 |
| `with pkgs;` scope pollution / 作用域污染 | `import pkgs (*)` explicit / 显式导入 |
| No type system / 无类型系统 | Full HM inference / 完整 HM 推导 |
| Lazy by default / 默认惰性 | Strict default, `lazy` keyword / 默认严格 |

## Name Origin / 命名由来

**Neve** means "snow" in Italian and Portuguese, connecting to Nix (Latin for "snow") while representing a fresh, clean design.

**Neve** 在意大利语和葡萄牙语中意为"雪"，与 Nix（拉丁语"雪"）相呼应，象征纯净、简洁的设计。

## License / 许可证

[MPL-2.0](LICENSE)

## Roadmap / 路线图

- [ ] Complete evaluator / 完善求值器 (lazy evaluation, imports / 惰性求值、导入)
- [ ] CLI tool / 命令行工具 (`neve eval`, `neve build`, `neve repl`)
- [ ] Sandbox builder / 沙箱构建器
- [ ] Store with GC / 带垃圾回收的存储
- [ ] Package repository / 包仓库
- [ ] System configuration / 系统配置
- [ ] Flake compatibility / Flake 兼容
