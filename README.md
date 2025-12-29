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
[![AUR](https://img.shields.io/aur/version/neve-git?color=1793d1)](https://aur.archlinux.org/packages/neve-git)

---

**[English](#english)** | **[中文](#中文)**

---

</div>

## English

> *Nix's soul. Better syntax. Type safety.*

### Why Neve?

| Pain Point | Nix | Neve |
|:-----------|:----|:-----|
| Is this a record or function? | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| Type errors | Runtime explosion | Compile-time catch |
| String interpolation | `"${x}"` | `` `{x}` `` |
| Recursion | `rec { ... }` | Just works |

### 30-Second Demo

```bash
$ neve repl
neve> #{ name = "world", greet = fn(n) `Hello, {n}!` }
#{greet = <fn>, name = "world"}
neve> let r = #{ name = "world", greet = fn(n) `Hello, {n}!` }
neve> r.greet(r.name)
"Hello, world!"
```

### Install

```bash
# Arch Linux
yay -S neve-git

# Pre-built binary
curl -fsSL https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv neve /usr/local/bin/

# From source
git clone https://github.com/MCB-SMART-BOY/neve && cd neve && cargo build --release
```

### Syntax at a Glance

```neve
-- Records (always #{ })
let config = #{ port = 8080, host = "localhost" };

-- Functions (always fn)
fn greet(name) = `Hello, {name}!`;

-- Pattern matching
fn factorial(n) = match n {
    0 -> 1,
    n -> n * factorial(n - 1),
};

-- Pipes
[1, 2, 3] |> map(fn(x) x * 2) |> filter(fn(x) x > 2)
```

### Documentation

```bash
neve doc              # List all topics
neve doc quickstart   # 5-minute guide
neve doc spec         # Language reference
neve doc api          # Standard library
```

### Project Status

| Component | Status |
|:----------|:-------|
| Language Core (lexer, parser, typeck, eval) | ✅ 95% |
| Toolchain (REPL, formatter, LSP) | ✅ 80% |
| Package Manager | 🚧 60% |
| System Configuration | 🚧 40% |

---

## 中文

> *Nix 的灵魂，更好的语法，类型安全。*

### 为什么选 Neve？

| 痛点 | Nix | Neve |
|:-----|:----|:-----|
| 这是记录还是函数？ | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| 类型错误 | 运行时爆炸 | 编译期捕获 |
| 字符串插值 | `"${x}"` | `` `{x}` `` |
| 递归 | `rec { ... }` | 自动处理 |

### 30 秒演示

```bash
$ neve repl
neve> #{ name = "世界", greet = fn(n) `你好，{n}！` }
#{greet = <fn>, name = "世界"}
neve> let r = #{ name = "世界", greet = fn(n) `你好，{n}！` }
neve> r.greet(r.name)
"你好，世界！"
```

### 安装

```bash
# Arch Linux
yay -S neve-git

# 下载预编译包
curl -fsSL https://github.com/MCB-SMART-BOY/neve/releases/latest/download/neve-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv neve /usr/local/bin/

# 从源码编译
git clone https://github.com/MCB-SMART-BOY/neve && cd neve && cargo build --release
```

### 语法一览

```neve
-- 记录（永远是 #{ }）
let config = #{ port = 8080, host = "localhost" };

-- 函数（永远是 fn）
fn greet(name) = `你好，{name}！`;

-- 模式匹配
fn factorial(n) = match n {
    0 -> 1,
    n -> n * factorial(n - 1),
};

-- 管道
[1, 2, 3] |> map(fn(x) x * 2) |> filter(fn(x) x > 2)
```

### 文档

```bash
neve doc              # 列出所有主题
neve doc quickstart   # 5 分钟入门
neve doc spec --zh    # 语言规范（中文）
neve doc api --zh     # 标准库（中文）
```

### 项目进度

| 组件 | 状态 |
|:-----|:-----|
| 语言核心（词法、语法、类型、求值） | ✅ 95% |
| 工具链（REPL、格式化、LSP） | ✅ 80% |
| 包管理器 | 🚧 60% |
| 系统配置 | 🚧 40% |

---

<div align="center">

**[Docs](docs/)** · **[Issues](https://github.com/MCB-SMART-BOY/neve/issues)** · **[License: MPL-2.0](LICENSE)**

*Made with ❄️ and mass amounts of ☕*

</div>
