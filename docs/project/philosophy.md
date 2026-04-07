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
#{ x = 1 }       -- ALWAYS a record
{ let x = 1; x } -- ALWAYS a block
fn(x) x + 1      -- ALWAYS a lambda
```

### 2. Syntax Unity / 语法统一

Similar concepts, similar syntax.

相似的东西，相似的写法。

| Concept / 概念 | Syntax / 语法 |
|---------------|---------------|
| Named function / 命名函数 | `fn add(x, y) = x + y;` |
| Lambda / Lambda | `fn(x, y) x + y` |
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

20 keywords total.

一共就 20 个关键字。

```
fn  let  if  then  else  match  import  as  self  super  crate  type
trait  impl  pub  true  false  struct  enum  lazy
```

### 6. Unix Philosophy / Unix 哲学

Do one thing well. Compose. Text is universal.

做好一件事。组合。文本是通用接口。

## Nix vs Neve / Nix 对比

| Pain Point / 槽点 | Nix | Neve |
|------------------|-----|------|
| Record or function? / 记录还是函数？ | `{ x = 1; }` vs `{ x }: x` | `#{ x = 1 }` vs `fn(x) x` |
| Type safety / 类型安全 | None / 没有 | Hindley-Milner / HM 类型推导 |
| Recursion / 递归 | `rec { }` | Automatic / 自动处理 |
| Inherit / 继承字段 | `inherit x y z;` | `#{ x, y, z }` |
| Error timing / 报错时机 | Runtime / 运行时 | Compile-time / 编译时 |

## Current Status / 开发进度

| Area / 领域 | Status | Notes / 说明 |
|------------|--------|-------------|
| Language core / 语言核心 | ⚠️ Broad but not fully converged | 语法表面较广，但 parser/lowering/typeck/eval 仍未完全收敛 |
| Runtime / 运行时 | ⚠️ Partially converged | AST 与 HIR 运行时并存，效果边界仍未正式划清 |
| Toolchain / 工具链 | ⚠️ Usable but uneven | REPL、formatter、diagnostics、LSP 已有基础，但语义一致性还在补 |
| Package system / 包管理 | 🚧 In progress | Fetch/store/derive/builder integration / 获取、存储、派生、构建整合 |
| OS integration / 系统集成 | ⚠️ Prototype | System configuration modules exist in prototype form / 系统配置模块已有原型 |

### Focus Now / 当前重点

- Module loader polish, LSP enhancements, diagnostics UX
- Package system integration (derivation/store/builder)
- 模块加载打磨、LSP 增强、诊断体验优化
- 包管理集成（derivation/store/builder）

### Future / 未来

- System configuration, macros, binary cache, HKT, Neve OS
- 系统配置、宏、二进制缓存、高阶类型、Neve OS

## The Name / 名字由来

*Neve* = "snow" in Italian/Portuguese. Same spirit as Nix (Latin "snow"), new journey.

Neve 是意大利语和葡萄牙语的「雪」，跟 Nix（拉丁语的「雪」）同源。同样的精神，新的旅程。

---

> "Perfection is achieved when there is nothing left to take away." / 「完美不是无可增加，而是无可删减。」
