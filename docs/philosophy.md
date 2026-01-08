```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                           DESIGN PHILOSOPHY                                   ║
║                              设计哲学                                          ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  Vision / Principles / Nix vs Neve / Status      │
│  [中文]     #chinese   ──→  愿景 / 设计原则 / 对比 Nix / 进度               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

<a name="english"></a>

# English

## The Vision

*"I don't hate Nix. I want to BE Nix — but the Nix that could have been, if we started fresh today."*

Neve inherits Nix's soul (purity, reproducibility, declarative) but sheds its legacy. No compatibility with nixpkgs. No compromises. A clean slate.

```
╔══════════════════════════════════════════════════════════════════════════════╗
║  "Inherit and surpass" — that's the goal.                                    ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

## Core Principles

### 1. Zero Ambiguity

Every syntax has one meaning. No guessing.

```neve
#{ x = 1 }       -- ALWAYS a record
{ let x = 1; x } -- ALWAYS a block
fn(x) x + 1      -- ALWAYS a lambda
```

### 2. Syntax Unity

Similar concepts, similar syntax.

| Concept | Syntax |
|---------|--------|
| Named function | `fn add(x, y) = x + y;` |
| Lambda | `fn(x, y) x + y` |
| Function type | `Int -> Int` |
| Match arm | `pattern -> result` |

Arrow (`→`) always means "produces".

### 3. No Magic Indentation

Explicit delimiters. No Python-style whitespace sensitivity.

### 4. Pure Functional

No side effects. No mutable state. Same input → same output. Always.

### 5. Simplicity

20 keywords total:

```
fn  let  if  then  else  match  import  as  self  super  crate  type
trait  impl  pub  true  false  struct  enum  lazy
```

### 6. Unix Philosophy

Do one thing well. Compose. Text is universal.

## Nix vs Neve

| Pain Point | Nix | Neve |
|------------|-----|------|
| Record or function? | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| Type safety | None | Hindley-Milner |
| Recursion | `rec { }` | Automatic |
| Inherit | `inherit x y z;` | `#{ x, y, z }` |
| Error timing | Runtime | Compile-time |

## Current Status

```
Language Core     [████████████████████░░]  95%
Toolchain         [███████████████░░░░░]    75%
Package System    [███████████░░░░░░░░░]    55%
OS Integration    [██████░░░░░░░░░░░░░░]    25%
```

### Completed

- Lexer, Parser, Type Checker, Evaluator
- REPL, Formatter, Diagnostics
- Standard library baseline (list/string/math/etc.)

### In Progress

- Module loader, Associated types, LSP enhancements, TCO
- Package system integration (derivation/store/builder)

### Future

- System configuration, Macros, Binary cache, HKT, Neve OS

## The Name

*Neve* = "snow" in Italian/Portuguese. Same spirit as Nix (Latin "snow"), new journey.

---

<a name="chinese"></a>

# 中文

## 愿景

说实话，我不讨厌 Nix。我想成为 Nix——但是是那个如果今天从头开始的话，本应该成为的 Nix。

Neve 继承了 Nix 的核心理念（纯函数、可复现、声明式），但甩掉了历史包袱。不兼容 nixpkgs，不妥协，从头来过。

```
╔══════════════════════════════════════════════════════════════════════════════╗
║  继承，然后超越。                                                            ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

## 设计原则

### 1. 零歧义

每种语法只有一个意思，不用猜。

```neve
#{ x = 1 }       -- 肯定是记录
{ let x = 1; x } -- 肯定是代码块
fn(x) x + 1      -- 肯定是 lambda
```

### 2. 语法统一

相似的东西，相似的写法。

| 概念 | 语法 |
|------|------|
| 命名函数 | `fn add(x, y) = x + y;` |
| Lambda | `fn(x, y) x + y` |
| 函数类型 | `Int -> Int` |
| 匹配分支 | `pattern -> result` |

箭头（→）永远表示「产出」。

### 3. 不靠缩进

有明确的分隔符，不像 Python 那样对空格敏感。

### 4. 纯函数

没有副作用，没有可变状态。相同输入永远得到相同输出。

### 5. 简洁

一共就 20 个关键字：

```
fn  let  if  then  else  match  import  as  self  super  crate  type
trait  impl  pub  true  false  struct  enum  lazy
```

### 6. Unix 哲学

做好一件事。组合。文本是通用接口。

## 跟 Nix 比

| 槽点 | Nix | Neve |
|------|-----|------|
| 记录还是函数？ | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| 类型安全 | 没有 | HM 类型推导 |
| 递归 | 要写 `rec { }` | 自动处理 |
| 继承字段 | `inherit x y z;` | `#{ x, y, z }` |
| 报错时机 | 运行时 | 编译时 |

## 开发进度

```
语言核心     [████████████████████░░]  95%
工具链       [███████████████░░░░░]    75%
包管理       [███████████░░░░░░░░░]    55%
系统配置     [██████░░░░░░░░░░░░░░]    25%
```

### 已完成

- 词法分析、语法分析、类型检查、求值器
- REPL、格式化、诊断系统
- 标准库基础模块（list/string/math 等）

### 进行中

- 模块加载、关联类型、LSP 增强、尾调用优化
- 包管理集成（derivation/store/builder）

### 未来

- 系统配置、宏系统、二进制缓存、高阶类型、Neve OS

## 名字由来

Neve 是意大利语和葡萄牙语的「雪」，跟 Nix（拉丁语的「雪」）同源。同样的精神，新的旅程。

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
     "Perfection is achieved when there is nothing left to take away."
                   「完美不是无可增加，而是无可删减。」
═══════════════════════════════════════════════════════════════════════════════
```

</div>
