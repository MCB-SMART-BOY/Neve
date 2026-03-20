<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Documentation Hub</h1>

<p><em>文档中心</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong> ·
  <strong><a href="quickstart.md">Quickstart</a></strong> ·
  <strong><a href="spec.md">Spec</a></strong> ·
  <strong><a href="api.md">API</a></strong>
</p>

</div>

Welcome to the Neve language documentation. 欢迎来到 Neve 语言的文档中心！

---

## Documentation Structure / 文档结构

| Document | Description | 描述 |
|----------|-------------|------|
| [quickstart.md](quickstart.md) | 5-minute quick start | 5分钟快速入门 |
| [onboarding.md](onboarding.md) | Contributor onboarding | 贡献者入门 |
| [tutorial.md](tutorial.md) | Complete tutorial | 完整教程 |
| [spec.md](spec.md) | Language specification | 语言规范 |
| [api.md](api.md) | Standard library reference | 标准库参考 |
| [diagnostics.md](diagnostics.md) | Diagnostic codes | 诊断错误码 |
| [philosophy.md](philosophy.md) | Design philosophy & roadmap | 设计哲学与路线图 |
| [roadmap.md](roadmap.md) | Project roadmap | 项目路线图 |
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
Int, Float, Bool, Char, String, Unit

-- Path literals currently evaluate to String.
-- 路径字面量目前会当成 String 处理。

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

## Binary Cache & Signatures / 二进制缓存与签名

```bash
# One key for all caches / 单个密钥应用到全部缓存
neve build ./pkg.neve \
  --cache-url https://cache.example.org \
  --cache-dir /var/lib/neve/cache \
  --cache-public-key 'ed25519:<base64-public-key>' \
  --cache-private-key 'ed25519:<base64-private-key>' \
  --cache-upload
```

Environment variables / 环境变量：

- `NEVE_BINARY_CACHE_URLS` (comma-separated) / 远程缓存 URL（逗号分隔）
- `NEVE_BINARY_CACHE_LOCAL_DIRS` (comma-separated) / 本地缓存目录（逗号分隔）
- `NEVE_BINARY_CACHE_PUBLIC_KEYS` (comma-separated, fallback `NEVE_BINARY_CACHE_PUBLIC_KEY`)
- `NEVE_BINARY_CACHE_PRIVATE_KEYS` (comma-separated, fallback `NEVE_BINARY_CACHE_PRIVATE_KEY`)
- `NEVE_BINARY_CACHE_UPLOAD` (`true/false`)
- `NEVE_SUBSTITUTE` (`true/false`)

Key mapping rule / 密钥映射规则：

- `0` keys: disabled / 禁用
- `1` key: apply to all cache sources / 应用于所有缓存源
- `N` keys: must match total cache sources / 必须等于缓存源总数

Behavior notes / 行为说明：

- Push writes `narinfo` with `FileHash`, `NarHash`, and `References`.
- Reference discovery is best-effort by scanning store payloads for existing store path tokens.
- Fetching a cached path resolves `References` recursively before installing the target path.
- If a referenced path is missing in caches, fetch fails to avoid partial closure installs.
- Remote retry only applies to transient failures (`429`, `5xx`, timeout/connect errors).

---

<div align="center">

**[Main README](../README.md)** · **[License: MPL-2.0](../LICENSE)**

</div>
