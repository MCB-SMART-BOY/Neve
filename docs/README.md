# Neve Documentation Hub / 文档中心

Welcome to the Neve language documentation!

欢迎来到 Neve 语言的文档中心！

---

## Documentation Structure / 文档结构

| Document | Description | 描述 |
|----------|-------------|------|
| [quickstart.md](quickstart.md) | 5-minute quick start | 5分钟快速入门 |
| [tutorial.md](tutorial.md) | Complete tutorial | 完整教程 |
| [spec.md](spec.md) | Language specification | 语言规范 |
| [api.md](api.md) | Standard library reference | 标准库参考 |
| [philosophy.md](philosophy.md) | Design philosophy & roadmap | 设计哲学与路线图 |
| [install.md](install.md) | Installation guide | 安装指南 |
| [architecture.md](architecture.md) | Internal architecture | 内部架构 |
| [changelog.md](changelog.md) | Version changelog | 版本更新日志 |

---

## Quick Start / 快速开始

```bash
# Install / 安装
cargo install neve

# Start REPL / 启动 REPL
neve repl

# Evaluate expression / 求值表达式
neve eval "1 + 2"

# Type check / 类型检查
neve check file.neve

# Format code / 格式化代码
neve fmt file.neve

# View documentation / 查看文档
neve doc quickstart
```

---

## Syntax Cheat Sheet / 语法速查

| Concept | Syntax | Example |
|---------|--------|---------|
| Record / 记录 | `#{ }` | `#{ x = 1, y = 2 }` |
| List / 列表 | `[ ]` | `[1, 2, 3]` |
| Lambda | `fn(x) expr` | `fn(x) x + 1` |
| Function / 函数 | `fn name(x) = expr;` | `fn add(a, b) = a + b;` |
| Pipe / 管道 | `\|>` | `x \|> f \|> g` |
| Interpolation / 插值 | `` `{expr}` `` | `` `sum = {1 + 2}` `` |
| Comment / 注释 | `-- --` | `-- this is a comment --` |
| Block / 代码块 | `{ }` | `{ let x = 1; x }` |
| Match / 匹配 | `match x { }` | `match x { 0 -> "zero", _ -> "other" }` |

---

## Type System / 类型系统

```neve
-- Primitive types / 原始类型
Int, Float, Bool, Char, String, Unit, Path

-- Compound types / 复合类型
List<Int>                     -- List / 列表
Option<Int>                   -- Optional / 可选
Result<Int, String>           -- Result / 结果
(Int, String)                 -- Tuple / 元组
Int -> Int                    -- Function / 函数
#{ name: String, age: Int }   -- Record / 记录
```

---

## Project Status / 项目进度

| Component | Status | Description |
|-----------|--------|-------------|
| Lexer & Parser | ✅ Complete | Full syntax support |
| Type Checker | ✅ Complete | Hindley-Milner + Traits |
| Evaluator | ✅ Complete | Lazy evaluation + TCO |
| REPL | ✅ Complete | Interactive development |
| Formatter | ✅ Complete | Opinionated formatting |
| LSP | 🚧 In Progress | Editor integration |
| Package Manager | 🚧 In Progress | Dependency management |
| System Config | 📋 Planned | NixOS-style configuration |

---

## Community / 社区

- **GitHub**: [MCB-SMART-BOY/Neve](https://github.com/MCB-SMART-BOY/Neve)
- **Issues**: [Bug reports & feature requests](https://github.com/MCB-SMART-BOY/Neve/issues)
- **Contributing**: [CONTRIBUTING.md](../CONTRIBUTING.md)

---

## CLI Commands / 命令行

| Command | Description | 描述 |
|---------|-------------|------|
| `neve repl` | Interactive REPL | 交互式 REPL |
| `neve eval <expr>` | Evaluate expression | 求值表达式 |
| `neve run <file>` | Run a file | 运行文件 |
| `neve check <file>` | Type check | 类型检查 |
| `neve fmt <file>` | Format code | 格式化代码 |
| `neve doc [topic]` | View documentation | 查看文档 |
| `neve info --platform` | Platform capabilities | 平台功能 |

---

<div align="center">

**[Main README](../README.md)** · **[License: MPL-2.0](../LICENSE)**

</div>
