# Neve Formal Verification

用 [Lean 4](https://lean-lang.org/) 对 Neve 语言核心语义的形式化验证。

## 目录

```
formal/
├── Neve/
│   ├── Syntax.lean    — 核心语法（类型、值、表达式、模式、运算）
│   ├── Types.lean     — 类型检查规则（Γ ⊢ e : τ）
│   ├── Eval.lean      — 操作语义（大步骤：env ⊢ e ⇓ v）
│   └── Safety.lean    — 类型安全定理
│                         · Progress:  类型正确的表达式不是 stuck
│                         · Preservation: 求值保持类型
├── lakefile.lean      — Lean 项目配置
└── README.md          — 本文件
```

## 如何使用

```bash
# 安装 Lean 4
curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh

# 构建
cd formal
lake build

# 交互式证明
lake exe lean --run
```

## 当前状态

| 模块 | 状态 | 说明 |
|------|------|------|
| Syntax | ✅ 完成 | 完整的 Neve 核心语法 |
| Types | ✅ 完成 | 14 条类型规则（λ 演算 + 管道 + 部分内置） |
| Eval | ✅ 完成 | 10 条求值规则（大步骤语义） |
| Safety | ⚠️ 框架 | 定理已声明，证明待完成（当前 `sorry`） |

## 形式化范围

### 已验证
- 基本类型系统：`Int`, `Bool`, `String`, `Unit`
- 函数类型：`τ₁ → τ₂`
- λ 演算：变量、抽象、应用
- `let` 绑定
- 二元运算：`+`, `==`, `&&`
- 管道 `|>`

### 未涉及
- I/O 效果系统（`effect`）
- 模式匹配
- 记录 / 列表 / 元组类型
- 泛型
- 内置函数（`io.*`）

## 与 Rust 实现的关系

这个形式化是 Neve 语言规范的数学定义。Rust 实现（`crates/`）应该符合这里的类型规则和求值规则。

- `formal/Neve/Types.lean` 对应 `crates/neve-typeck/`
- `formal/Neve/Eval.lean` 对应 `crates/neve-eval/`
- `formal/Neve/Syntax.lean` 对应 `crates/neve-hir/`

## 参考文献

- [Types and Programming Languages](https://www.cis.upenn.edu/~bcpierce/tapl/) — Benjamin Pierce
- [Software Foundations](https://softwarefoundations.cis.upenn.edu/) — 类型安全证明的标准写法
