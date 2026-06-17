<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

<h1>Design Philosophy</h1>

<p><em>设计哲学</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

---

## The Vision / 愿景

"I don't hate Nix. I want to BE Nix — but the Nix that could have been, if we started fresh today."

说实话，我不讨厌 Nix。我想成为 Nix——但是是那个如果今天从头开始的话，本应该成为的 Nix。

Neve inherits Nix's soul (purity, reproducibility, declarative) but sheds its legacy. No compatibility with nixpkgs. No compromises. A clean slate.

Neve 继承了 Nix 的核心理念（纯函数、可复现、声明式），但甩掉了历史包袱。不兼容 nixpkgs，不妥协，从头来过。

> "Inherit and surpass" — that's the goal. / 继承，然后超越。

## Core Principles / 设计原则

### 1. Zero Ambiguity / 零歧义

Every syntax has one meaning. No guessing.

每种语法只有一个意思，不用猜。

```neve
{ x = 1 }       -- ALWAYS a record (container delimited by { })
{ x = 1; x }    -- ALWAYS a block
|x| x + 1       -- ALWAYS a lambda
```

### 2. Syntax Unity / 语法统一

Similar concepts, similar syntax.

相似的东西，相似的写法。

| Concept / 概念 | Syntax / 语法 |
|---------------|---------------|
| Named function / 命名函数 | `add(x, y) = x + y` |
| Lambda / Lambda | `|x, y| x + y` |
| Function type / 函数类型 | `Int -> Int` |
| Match arm / 匹配分支 | `pattern -> result` |

Arrow (`->`) always means "produces".

箭头（`->`）永远表示「产出」。

### 3. No Magic Indentation / 不靠缩进

Explicit delimiters. No Python-style whitespace sensitivity.

有明确的分隔符，不像 Python 那样对空格敏感。

### 4. Pure Functional / 纯函数

No side effects. No mutable state. Same input → same output. Always.

没有副作用，没有可变状态。相同输入永远得到相同输出。

### 5. Simplicity / 简洁

A minimal set of keywords (v4.0: 12 canonical keywords, down from 21 in v1.x).

精简的关键字集合（v4.0: 12 个规范关键字，v1.x 时为 21 个）。

```
let  fn   type  trait  impl  use
self if   else  match
true false
```

### 6. Unix Philosophy / Unix 哲学

Do one thing well. Compose. Text is universal.

做好一件事。组合。文本是通用接口。

## Nix vs Neve / Nix 对比

| Pain Point / 槽点 | Nix | Neve |
|------------------|-----|------|
| Record or function? / 记录还是函数？ | `{ x = 1; }` vs `{ x }: x` | `{ x = 1 }` vs `|x| x` |
| Type safety / 类型安全 | None / 没有 | Hindley-Milner / HM 类型推导 |
| Recursion / 递归 | `rec { }` | Automatic / 自动处理 |
| Inherit / 继承字段 | `inherit x y z;` | `{ x, y, z }` |
| Error timing / 报错时机 | Runtime / 运行时 | Compile-time / 编译时 |

## Current Status / 开发进度

| Area / 领域 | Status | Notes / 说明 |
|------------|--------|-------------|
| Language core / 语言核心 | ✅ Converged | v4.0 syntax canonical, parser/lowering/typeck/eval 全链路闭环 |
| Runtime / 运行时 | ✅ HIR canonical | AST compat path removed, all eval goes through HIR |
| Toolchain / 工具链 | ✅ Usable | REPL, formatter, diagnostics, LSP (20 methods), all CI green |
| Package system / 包管理 | ✅ Implemented | Fetch/store/derive/builder/registry, binary cache with narinfo signing |
| OS integration / 系统集成 | ⚠️ Linux-only | System configuration and sandbox are Linux-native; Docker backend for macOS |

### Focus Now / 当前重点

- Registry public launch (`registry.neve.dev`)
- Cross-platform sandbox improvements
- Standard library expansion
- 注册表公开发布（`registry.neve.dev`）
- 跨平台沙箱改进
- 标准库扩展

### Future / 未来

- HKT, macros, Neve OS, decentralized package federation
- 高阶类型、宏、Neve OS、去中心化包联邦

## The Name / 名字由来

*Neve* = "snow" in Italian/Portuguese. Same spirit as Nix (Latin "snow"), new journey.

Neve 是意大利语和葡萄牙语的「雪」，跟 Nix（拉丁语的「雪」）同源。同样的精神，新的旅程。

---

> "Perfection is achieved when there is nothing left to take away." / 「完美不是无可增加，而是无可删减。」
