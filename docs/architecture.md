# Neve Architecture / 架构设计

This document describes the internal architecture of Neve for contributors and developers.

本文档为贡献者和开发者描述 Neve 的内部架构。

---

## Overview / 概述

Neve is a pure functional language designed for system configuration and package management. The codebase is organized as a Cargo workspace with 16 modular crates.

Neve 是一门为系统配置和包管理设计的纯函数式语言。代码库组织为包含 16 个模块化 crate 的 Cargo workspace。

```
┌─────────────────────────────────────────────────────────────────────┐
│                           neve-cli                                   │
│                    (Command-line interface)                          │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  neve-config  │    │   neve-builder  │    │    neve-lsp     │
│ (System Cfg)  │    │ (Package Build) │    │ (Language Srv)  │
└───────┬───────┘    └────────┬────────┘    └────────┬────────┘
        │                     │                      │
        └──────────┬──────────┴──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                           neve-eval                                  │
│                    (Interpreter / Evaluator)                         │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          neve-typeck                                 │
│                    (Type Checker / Inference)                        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                           neve-hir                                   │
│                (High-level Intermediate Repr)                        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         neve-parser                                  │
│                    (Syntax Analysis / AST)                           │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          neve-lexer                                  │
│                    (Tokenization / Lexing)                           │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Crate Responsibilities / Crate 职责

### Core Language Pipeline / 核心语言管线

| Crate | Responsibility | 职责 |
|-------|----------------|------|
| `neve-lexer` | Tokenization using logos | 使用 logos 进行词法分析 |
| `neve-syntax` | AST node definitions | AST 节点定义 |
| `neve-parser` | Recursive descent parser (LL(1)) | 递归下降解析器 (LL(1)) |
| `neve-hir` | High-level IR with name resolution | 高级中间表示与名称解析 |
| `neve-typeck` | Hindley-Milner type inference | Hindley-Milner 类型推导 |
| `neve-eval` | Tree-walking interpreter | 树遍历解释器 |
| `neve-std` | Standard library modules | 标准库模块 |

### Tooling / 工具链

| Crate | Responsibility | 职责 |
|-------|----------------|------|
| `neve-cli` | Command-line interface | 命令行界面 |
| `neve-lsp` | Language Server Protocol | 语言服务器协议 |
| `neve-fmt` | Code formatter | 代码格式化器 |
| `neve-diagnostic` | Error reporting with ariadne | 使用 ariadne 的错误报告 |

### Package Management / 包管理

| Crate | Responsibility | 职责 |
|-------|----------------|------|
| `neve-store` | Content-addressed storage (BLAKE3) | 内容寻址存储 (BLAKE3) |
| `neve-fetch` | Source fetching (URL, Git, local) | 源码获取 (URL, Git, 本地) |
| `neve-builder` | Sandbox build system | 沙箱构建系统 |
| `neve-derive` | Derivation model & hashing | Derivation 模型与哈希 |
| `neve-config` | System configuration | 系统配置 |

### Shared / 共享

| Crate | Responsibility | 职责 |
|-------|----------------|------|
| `neve-common` | Shared utilities & types | 共享工具与类型 |

---

## Data Flow / 数据流

```
Source Code (.neve)
       │
       ▼
┌──────────────┐
│   Lexer      │  Tokens: [(Token, Span), ...]
│  (logos)     │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Parser     │  AST: Module { items: [...] }
│   (LL(1))    │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  HIR Lower   │  HIR: Module with resolved names
│  (resolve)   │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Type Check  │  Typed HIR: expressions with types
│  (HM + Traits)│
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Evaluator   │  Value: Int | String | List | ...
│ (tree-walk)  │
└──────────────┘
```

---

## Key Design Decisions / 关键设计决策

### 1. Lazy Evaluation / 惰性求值

Neve uses lazy evaluation by default. Values are wrapped in thunks and only forced when needed.

Neve 默认使用惰性求值。值被包装在 thunk 中，仅在需要时才强制求值。

```rust
enum Value {
    Int(i64),
    Thunk(Rc<RefCell<ThunkState>>),
    // ...
}

enum ThunkState {
    Pending(Expr, Env),
    Forced(Value),
}
```

### 2. Content-Addressed Storage / 内容寻址存储

All build outputs are stored by their content hash (BLAKE3), enabling:
- Reproducible builds
- Efficient caching
- Deduplication

所有构建输出按内容哈希 (BLAKE3) 存储，实现：
- 可重现构建
- 高效缓存
- 去重

```
/neve/store/
├── abc123.../ -> actual files
├── def456.../ -> actual files
└── ...
```

### 3. Sandbox Isolation / 沙箱隔离

Builds run in isolated environments using Linux namespaces:
- User namespace (unprivileged)
- Mount namespace (isolated filesystem)
- Network namespace (optional isolation)

构建在使用 Linux 命名空间的隔离环境中运行：
- 用户命名空间（非特权）
- 挂载命名空间（隔离文件系统）
- 网络命名空间（可选隔离）

### 4. Hindley-Milner + Traits / HM + Trait

Type system features:
- Complete type inference
- Parametric polymorphism
- Trait constraints (like Rust/Haskell)
- Row polymorphism for records

类型系统特性：
- 完整类型推导
- 参数多态
- Trait 约束（类似 Rust/Haskell）
- 记录的行多态

---

## Platform Support / 平台支持

| Feature | Linux | macOS | Windows |
|---------|-------|-------|---------|
| Language Core | ✅ | ✅ | ✅ |
| REPL | ✅ | ✅ | ✅ |
| LSP | ✅ | ✅ | ✅ |
| Native Sandbox | ✅ | ❌ | ❌ |
| Docker Backend | ✅ | ✅ | ✅ |
| System Config | ✅ | ❌ | ❌ |

---

## Directory Structure / 目录结构

```
Neve/
├── neve-cli/                 # CLI application
│   └── src/
│       ├── main.rs          # Entry point
│       ├── commands/        # Subcommands
│       └── output.rs        # Formatting utilities
├── crates/
│   ├── neve-lexer/          # Tokenizer
│   ├── neve-parser/         # Parser
│   ├── neve-syntax/         # AST types
│   ├── neve-hir/            # HIR & resolution
│   ├── neve-typeck/         # Type checker
│   ├── neve-eval/           # Interpreter
│   ├── neve-std/            # Standard library
│   ├── neve-store/          # Content store
│   ├── neve-fetch/          # Source fetching
│   ├── neve-builder/        # Build system
│   ├── neve-config/         # System config
│   ├── neve-lsp/            # Language server
│   ├── neve-fmt/            # Formatter
│   ├── neve-diagnostic/     # Error display
│   ├── neve-common/         # Shared types
│   └── neve-derive/         # Derivations
├── docs/                    # Documentation
├── tests/                   # Integration tests
└── examples/                # Example code
```

---

## Build Profiles / 构建配置

### Development / 开发

```bash
cargo build              # Fast iteration
cargo test               # Run tests
cargo clippy --workspace # Lint all crates
```

### Release / 发布

```bash
cargo build --release    # Optimized binary with LTO
```

Release profile settings:
- LTO (Link Time Optimization)
- Strip symbols
- Single codegen unit
- Panic = abort

---

## Testing Strategy / 测试策略

1. **Unit Tests**: In-module `#[test]` functions
2. **Integration Tests**: `/tests/` directory
3. **Example Programs**: `/examples/` directory
4. **CI Matrix**: Linux, macOS, Windows + cross-compilation

---

## Contributing Areas / 贡献方向

### High Priority / 高优先级
- LSP completion & hover
- Package dependency resolution
- Type error messages

### Medium Priority / 中优先级
- Pattern match compilation
- Remote build cache
- Performance profiling

### Lower Priority / 低优先级
- Windows system config
- macOS native sandbox
- Additional standard library modules

---

## Further Reading / 延伸阅读

- [Language Specification](spec.md)
- [API Reference](api.md)
- [Tutorial](tutorial.md)
- [Philosophy](philosophy.md)
