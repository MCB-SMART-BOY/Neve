# Neve Design Philosophy / Neve 设计哲学

---

## English

### Project Vision

Neve is a pure functional language designed for Unix-like operating system configuration and package management. The goal is to fully replace the Nix language in NixOS while addressing Nix's historical baggage and design flaws.

Neve serves as a core component for Unix-like operating systems, handling:
- System configuration management
- Package definition and building
- Environment isolation and reproducibility

---

### Core Design Principles

#### 1. Zero Ambiguity

**Every syntactic construct must have exactly one way to parse.**

This means:
- The parser can be a simple LL(1), no backtracking needed
- Code meaning is deterministic for both humans and machines
- No reliance on context to guess user intent

Implementation:
- Records use `#{ }` instead of `{ }`, completely separated from code blocks
- Closures use `fn(x) expr` instead of `x -> expr`, separated from type signatures
- Lists use `[ ]`, generics use `< >`, never mixed
- Comments use `-- --`, no conflict with any operator

#### 2. Syntax Unification

**Similar concepts use similar syntax.**

| Concept | Syntax | Consistency |
|---------|--------|-------------|
| Named function | `fn name(x) expr` | `fn` keyword |
| Anonymous function | `fn(x) expr` | `fn` keyword |
| Function type | `A -> B` | `->` arrow |
| Match branch | `pattern -> expr` | `->` arrow |
| Type declaration | `x: Int` | `:` colon |
| Value binding | `x = 1` | `=` equals |

#### 3. Indentation Independent

**All structures have explicit boundary markers.**

- Code blocks wrapped with `{ }`
- Statements terminated with `;`
- List items separated by `,`
- Definitions end with `;`

This ensures:
- Code formatting doesn't affect semantics
- Copy-paste doesn't break programs
- Parser implementation is simple and reliable

#### 4. Pure Functional

**No side effects, no mutable state.**

- All functions are pure
- Same input always produces same output
- Side effects through build descriptions (derivations), not direct execution

Benefits:
- Perfect reproducibility
- Safe parallel builds
- Reliable caching

#### 5. Simplicity First

**Minimize syntactic noise without sacrificing clarity.**

- 17 keywords, enough to express all concepts
- Record field shorthand: `#{ name }` equals `#{ name = name }`
- Type inference reduces redundant annotations
- Pipeline operator `|>` makes data flow clear

#### 6. Unix Philosophy

**Inherit Unix design wisdom.**

- Small and focused: each module does one thing well
- Composition over inheritance: build complex logic through pipes and function composition
- Text as universal interface: config, paths, commands are all processable data
- Silence is golden: no noise on success

---

### Differences from Nix

#### Nix's Problems

1. **Syntax ambiguity**: `{ a = 1; }` is both a record and potentially a function body
2. **Indentation traps**: indentation affects parsing in some cases
3. **Implicit behavior**: `rec { }` recursion, `with` scope pollution
4. **Symbol confusion**: `//` is merge, but looks like a comment
5. **Historical baggage**: design decisions hard to change

#### Neve's Solutions

| Nix Problem | Neve Solution |
|-------------|---------------|
| `{ }` polysemy | `#{ }` for records, `{ }` for code blocks |
| `x: x+1` closures | `fn(x) x+1` explicit |
| `rec { }` explicit recursion | Automatic detection, no marking needed |
| `with pkgs;` implicit import | `import pkgs (*)` explicit |
| `inherit x;` | `#{ x }` shorthand |
| `;` ending required | `;` ending consistent |

---

### Implementation Principles

#### Pure Rust Implementation

The entire language toolchain is implemented in pure Rust, with no C/C++ dependencies:

- **Lexer**: Hand-written with `logos`
- **Parser**: Hand-written recursive descent (LL(1) sufficient)
- **Type Checker**: Hindley-Milner algorithm
- **Evaluator**: Tree-walking interpreter
- **Package Management**: Content-addressed store

Why Rust:
- Memory safe, no GC pauses
- Excellent error handling
- Cross-platform compilation
- Suitable for systems programming

#### Error Messages First

Compiler error messages must:
- Point to exact error location
- Explain why it's an error
- Suggest how to fix

#### Incremental Implementation

Prioritized phased implementation:

1. **Phase 1**: Core language (lexer, parser, type checking, evaluation) ✓
2. **Phase 2**: Package management (derivation, store, builder) ✓
3. **Phase 3**: System configuration (module system, config generation) ✓
4. **Phase 4**: Tooling (LSP, formatter, REPL) ✓

---

### Roadmap

#### Completed ✅

- [x] Lexer (logos-based, full Unicode support)
- [x] Parser (recursive descent LL(1) with error recovery)
- [x] Type inference (Hindley-Milner with trait constraints)
- [x] Interpreter (tree-walking with lazy evaluation)
- [x] Standard library (io, list, map, math, option, path, result, set, string)
- [x] Derivation model (with hash verification)
- [x] Content-addressed store
- [x] Sandboxed builder (Linux namespaces)
- [x] Source fetching (URL, Git, local paths)
- [x] Language Server Protocol support (semantic tokens, symbols)
- [x] Code formatter
- [x] Interactive REPL
- [x] System configuration framework
- [x] Generation management
- [x] Module system (import, export, visibility)
- [x] Trait system (definitions, impls, resolution)

#### In Progress 🔄

- [ ] Module loader refinement
- [ ] Associated types in traits
- [ ] LSP enhancements (go-to-definition, completion)
- [ ] Tail call optimization

#### Future 📋

- [ ] Macro system
- [ ] Bootstrap package set
- [ ] Binary cache service
- [ ] Higher-kinded types

#### Operating System Integration

Neve is designed to become the core of a Unix-like operating system:

- **System Configuration**: Declaratively define entire system state
- **Package Management**: All software built and managed through Neve
- **Environment Management**: Dev environments, containers, VM configs
- **Deployment**: Unified management from single machine to clusters

---

### Design Decision Records

#### Why `#{ }` instead of other symbols?

Considered alternatives:
- `@{ }` — `@` used for pattern binding, conflicts
- `%{ }` — viable, but `#` is more common
- `${ }` — conflicts with interpolation
- `#{ }` — no conflicts, precedent in Ruby etc.

#### Why `fn(x) expr` instead of `\x -> expr`?

- `\` may have display issues in terminals
- `fn` consistent with named functions
- Easier for beginners to understand
- Simpler parsing (`fn(` clearly starts a closure)

#### Why `-- --` comments?

- More concise than `/* */`
- Supports multiline unlike standalone `--`
- No conflict with any operator
- Visually symmetric

#### Why strict evaluation by default?

- Package builds need deterministic execution order
- Avoids space leaks from lazy evaluation
- More intuitive debugging
- Use `lazy` to explicitly mark deferred evaluation

---

### Name Origin

**Neve** means "snow" in multiple languages (Italian, Portuguese, etc.).

Chosen because:
- Short and memorable
- Spiritual connection to Nix (Latin for "snow")
- Suggests pure, clean design philosophy
- Not taken by other major projects

---

## 中文

### 项目愿景

Neve 是一门为类 Unix 操作系统设计的纯函数式系统配置与包管理语言，目标是完全平替 NixOS 中的 Nix 语言，同时解决 Nix 的历史包袱和设计缺陷。

Neve 将作为类 Unix 操作系统的核心组件，承担：
- 系统配置管理
- 包定义与构建
- 环境隔离与复现

---

### 核心设计原则

#### 1. 零二义性

**每一个语法构造都必须有且仅有一种解析方式。**

这意味着：
- 解析器可以是简单的 LL(1)，无需回溯
- 代码的含义对人和机器都是确定的
- 不依赖上下文猜测用户意图

实现方式：
- 记录用 `#{ }` 而非 `{ }`，与代码块完全区分
- 闭包用 `fn(x) expr` 而非 `x -> expr`，与类型签名区分
- 列表用 `[ ]`，泛型用 `< >`，绝不混用
- 注释用 `-- --`，不与任何操作符冲突

#### 2. 语法统一

**相似的概念使用相似的语法。**

| 概念 | 语法 | 一致性 |
|------|------|--------|
| 命名函数 | `fn name(x) expr` | `fn` 关键字 |
| 匿名函数 | `fn(x) expr` | `fn` 关键字 |
| 函数类型 | `A -> B` | `->` 箭头 |
| match 分支 | `pattern -> expr` | `->` 箭头 |
| 类型声明 | `x: Int` | `:` 冒号 |
| 值绑定 | `x = 1` | `=` 等号 |

#### 3. 不依赖缩进

**所有结构都有显式的边界标记。**

- 代码块用 `{ }` 包裹
- 语句用 `;` 终止
- 列表项用 `,` 分隔
- 定义用 `;` 结束

这保证了：
- 代码格式化不影响语义
- 复制粘贴不会破坏程序
- 解析器实现简单可靠

#### 4. 纯函数式

**无副作用，无可变状态。**

- 所有函数都是纯函数
- 相同输入永远产生相同输出
- 副作用通过构建描述（derivation）而非直接执行

这带来：
- 完美的可复现性
- 安全的并行构建
- 可靠的缓存机制

#### 5. 简洁优先

**在不牺牲清晰性的前提下，追求最少的语法噪音。**

- 17 个关键字，足够表达所有概念
- 记录字段可简写：`#{ name }` 等于 `#{ name = name }`
- 类型推导减少冗余注解
- 管道操作符 `|>` 让数据流清晰

#### 6. Unix 哲学

**继承 Unix 的设计智慧。**

- 小而专注：每个模块做好一件事
- 组合优于继承：通过管道和函数组合构建复杂逻辑
- 文本是通用接口：配置、路径、命令都是可处理的数据
- 沉默是金：成功时不输出废话

---

### 与 Nix 的区别

#### Nix 的问题

1. **语法二义性**：`{ a = 1; }` 既是记录也可能是函数体
2. **缩进敏感的陷阱**：某些情况下缩进影响解析
3. **隐式行为**：`rec { }` 的递归、`with` 的作用域污染
4. **符号混乱**：`//` 是合并，但看起来像注释
5. **历史包袱**：设计决策难以修改

#### Neve 的改进

| Nix 问题 | Neve 方案 |
|----------|-----------|
| `{ }` 多义 | `#{ }` 记录，`{ }` 代码块 |
| `x: x+1` 闭包 | `fn(x) x+1` 明确 |
| `rec { }` 显式递归 | 自动检测，无需标记 |
| `with pkgs;` 隐式导入 | `import pkgs (*)` 显式 |
| `inherit x;` | `#{ x }` 简写 |
| `;` 结尾必须 | `;` 结尾一致 |

---

### 实现原则

#### 纯 Rust 实现

整个语言工具链使用纯 Rust 实现，不依赖任何 C/C++ 库：

- **Lexer**：使用 `logos` 手写
- **Parser**：手写递归下降（LL(1) 足够）
- **类型检查**：Hindley-Milner 算法
- **求值器**：Tree-walking 解释器
- **包管理**：Content-addressed store

选择 Rust 的理由：
- 内存安全，无 GC 停顿
- 优秀的错误处理
- 可编译到任何平台
- 适合系统级编程

#### 错误信息优先

编译器的错误信息必须：
- 指出确切的出错位置
- 解释为什么这是错误
- 建议如何修复

#### 渐进式实现

按优先级分阶段实现：

1. **Phase 1**：核心语言（lexer、parser、类型检查、求值）✓
2. **Phase 2**：包管理（derivation、store、builder）✓
3. **Phase 3**：系统配置（模块系统、配置生成）✓
4. **Phase 4**：工具链（LSP、格式化器、REPL）✓

---

### 发展路线

#### 已完成 ✅

- [x] 词法分析器（基于 logos，完整 Unicode 支持）
- [x] 语法分析器（递归下降 LL(1)，错误恢复）
- [x] 类型推导（Hindley-Milner + Trait 约束）
- [x] 解释器（树遍历 + 惰性求值）
- [x] 标准库（io、list、map、math、option、path、result、set、string）
- [x] Derivation 模型（哈希验证）
- [x] Content-addressed store
- [x] 沙箱构建器（Linux 命名空间）
- [x] 源码获取（URL、Git、本地路径）
- [x] Language Server Protocol 支持（语义高亮、符号索引）
- [x] 代码格式化器
- [x] 交互式 REPL
- [x] 系统配置框架
- [x] 代际管理
- [x] 模块系统（import、export、可见性）
- [x] Trait 系统（定义、实现、解析）

#### 进行中 🔄

- [ ] 模块加载器完善
- [ ] Trait 关联类型
- [ ] LSP 增强（跳转定义、自动补全）
- [ ] 尾调用优化

#### 未来 📋

- [ ] 宏系统
- [ ] Bootstrap 包集合
- [ ] 二进制缓存服务
- [ ] 高阶类型 (HKT)

#### 操作系统集成

Neve 设计为类 Unix 操作系统的核心：

- **系统配置**：声明式定义整个系统状态
- **包管理**：所有软件通过 Neve 构建和管理
- **环境管理**：开发环境、容器、虚拟机配置
- **部署**：从单机到集群的统一管理

---

### 设计决策记录

#### 为什么用 `#{ }` 而不是其他符号？

考虑过的方案：
- `@{ }` — `@` 用于模式绑定，有冲突
- `%{ }` — 可行，但 `#` 更常见
- `${ }` — 与插值冲突
- `#{ }` — 无冲突，Ruby 等语言有先例

#### 为什么用 `fn(x) expr` 而不是 `\x -> expr`？

- `\` 在终端显示可能有问题
- `fn` 与命名函数一致
- 更容易被新手理解
- 解析更简单（`fn(` 明确开始闭包）

#### 为什么用 `-- --` 注释？

- 比 `/* */` 更简洁
- 比单独的 `--` 支持多行
- 不与任何操作符冲突
- 视觉上对称美观

#### 为什么默认严格求值？

- 包构建需要确定的执行顺序
- 避免惰性求值的空间泄漏
- 调试更直观
- 用 `lazy` 显式标记需要延迟的地方

---

### 命名由来

**Neve** 在多种语言中意为"雪"（意大利语、葡萄牙语等）。

选择这个名字是因为：
- 简短好记
- 与 Nix（拉丁语"雪"）有精神联系
- 暗示纯净、简洁的设计理念
- 未被其他主流项目占用

---

### License / 许可证

[MPL-2.0](LICENSE)
