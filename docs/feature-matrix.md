<div align="center">

<img src="../assets/logo.svg" width="120" alt="Neve logo">

<h1>Neve Feature Matrix</h1>

<p><em>真实功能支持矩阵（v0）</em></p>

<p>
  <strong><a href="../README.md">Home</a></strong> ·
  <strong><a href="./">Docs</a></strong> ·
  <strong><a href="language-roadmap.md">Language Roadmap</a></strong>
</p>

</div>

---

这份文档的目的只有一个：

**把“Neve 现在到底做到哪一步”写清楚。**

它不是宣传页，也不是愿景列表。
它记录的是当前仓库里的真实支持情况。

## 怎么看 / How To Read

### 状态标记

| 标记 | 含义 |
|------|------|
| `✅` | 这一层基本支持，当前没有明显语义断裂 |
| `⚠️` | 部分支持，或者有明显语义分叉、精度不足、工具链不一致 |
| `❌` | 当前基本不支持，或只能靠占位逻辑通过 |
| `N/A` | 这一层不适用 |

### 这份矩阵目前是什么

这是 `v0` 版本，先覆盖**最容易误判为“已经完成”**的高风险特性。

它当前重点回答：

- parser 接受了，不代表语言真的支持了
- AST evaluator 能跑，不代表 canonical runtime 已经闭环
- 文档写了，不代表工具链已经对齐

后续版本会继续扩展到更完整的 feature inventory。

## 总体判断 / High-Level Truth

当前项目的总体情况可以用三句话概括：

1. **语法表面比语义闭环走得更快。**
2. **AST 路径比 HIR 路径更完整，但长期不能以 AST 路径为唯一真相。**
3. **系统脚本能力已经起步，但离“替代 Bash”还差命令模型、管道模型和 effect boundary。**

## 语言高风险特性矩阵 / High-Risk Language Features

| Feature / 特性 | Parser | HIR Lowering | Type Check | AST Runtime | HIR Runtime | Tooling | 当前判断 |
|----------------|--------|--------------|------------|-------------|-------------|---------|----------|
| 基础字面量、算术、记录、列表、元组 | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | 核心表达式基本可用，但端到端与工具链统一性还不足 |
| 模块导入与模块图 | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | 模块系统已有实装，但 CLI 运行主路径仍偏 AST |
| 列表推导 | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | 语言层基本可用，工具链覆盖不足 |
| 安全字段访问 `?.` | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | 运行时可用，但类型语义仍偏弱 |
| 路径字面量 | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | 目前本质上仍被当成 `String`，不是独立 `Path` |
| 惰性表达式 `lazy` | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | `lazy/force/isLazy/isEvaluated` 已在 AST/HIR 路径闭环，工具链覆盖仍需继续补齐 |
| 空值合并 `??` | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | AST/HIR runtime 已对齐并支持 Option-like enum、`std.option` builtin 与 safe-field fallback；完整 optionality 类型模型仍需继续收敛 |
| 错误传播 `?` | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | AST/HIR runtime 已对齐并支持 Option/Result-like enum，以及 `std.option` / `std.result` builtin；完整类型与 effect 语义仍需继续收敛 |
| Trait 定义与 impl 完整性 | ✅ | ✅ | ⚠️ | N/A | N/A | ⚠️ | 声明和部分完整性检查存在，但还不能视为完全闭环 |
| 关联类型（声明与完整性） | ✅ | ✅ | ⚠️ | N/A | N/A | ⚠️ | 声明层有，真实 use-site 语义仍需补完 |
| 方法调用语法 `x.foo(y)` | ✅ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | AST/HIR runtime 都已可执行方法调用，CLI 主路径已优先走 HIR，impl 方法体也开始进入 typecheck；但 trait 签名一致性、关联类型 use-site 与完整方法模型仍需继续收敛 |
| Or pattern `a | b` | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | HIR lowering/typecheck/runtime 已收敛，工具链覆盖仍需继续补齐 |
| Binding pattern `x @ pat` | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | `name @ pattern` 已在 AST/HIR 路径闭环，工具链覆盖仍需继续补齐 |
| List rest pattern `[x, ..xs]` | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | `init/rest/tail` 语义已在 AST/HIR 路径闭环，工具链覆盖仍需继续补齐 |
| 记录模式匹配 | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | 语言形态存在，编译器级检查不足 |
| Match 穷尽性检查 | N/A | N/A | ⚠️ | N/A | N/A | ❌ | 现已接入 typecheck 主流程，支持 `Bool`、`Unit`、用户枚举，以及 builtin `Option/Result`；列表、记录和更复杂子模式仍需继续扩展 |
| Unreachable pattern 警告 | N/A | N/A | ⚠️ | N/A | N/A | ❌ | 现已支持“前置分支已完成总覆盖”后的不可达告警，包括不可反驳分支、布尔全覆盖、用户枚举全覆盖与 builtin `Option/Result` 全覆盖；更细粒度的子集判定仍需继续扩展 |
| REPL `:type` | N/A | N/A | N/A | N/A | N/A | ⚠️ | 已可查询常见表达式与当前 REPL 绑定的类型；对 builtin 精度和完整语义环境的支持仍需继续补强 |
| 真实端到端执行测试 | N/A | N/A | N/A | N/A | N/A | ❌ | 当前 `tests/end_to_end.rs` 还存在占位 helper |

## 工具链一致性矩阵 / Tooling Fidelity Matrix

| Area / 领域 | 现状 | 主要问题 |
|-------------|------|----------|
| `neve check` | ⚠️ 可用 | 类型检查能跑，但还不是编译器级闭环 |
| `neve eval` | ⚠️ 可用 | 无 `import` 输入、本地模块导入，以及常见 `std` item/module/glob 导入已默认走 frontend/HIR；仅少数仍未收敛的导入/运行时边缘场景会回退 AST |
| `neve run` | ⚠️ 可用 | 普通模块图和常见 `std` item/module/glob 导入已可走 HIR；真正的统一 canonical path 仍受少数边缘导入/运行时语义限制 |
| REPL | ⚠️ 可用 | 交互与 `:type` 都能工作，但当前类型查询仍依赖保守的临时类型环境，不是完整编译器级语义镜像 |
| Formatter | ⚠️ 基本可用 | 日常可用，但“稳定且幂等”还应继续验证 |
| LSP | ⚠️ 持续收敛中 | 前端管线已接入，但功能完整性与一致性仍在补 |
| End-to-end tests | ❌ 不可信 | 还不能作为“语言已闭环”的证据 |

## 系统脚本能力矩阵 / System Scripting Matrix

### 已经开始具备的能力

| Capability / 能力 | 当前状态 | 说明 |
|-------------------|----------|------|
| 文件读取 | ✅ | `std.io` 已支持 |
| 文件写入与追加 | ✅ | `io.writeFile` / `io.appendFile` 已支持 |
| 目录递归创建与删除 | ✅ | `io.createDirAll` / `io.removeDirAll` 已支持 |
| 环境变量读取 | ✅ | `io.getEnv` 已支持 |
| 进程执行并捕获输出 | ✅ | `io.exec` / `io.execShell` 已支持 |
| 可配置执行（`cwd` / `env` / `stdin`） | ✅ | `io.execWith` 已支持 |

### 仍然缺失的关键能力

| Capability / 能力 | 当前状态 | 为什么还不能叫“替代 Bash” |
|-------------------|----------|----------------------------|
| 一等 `Path` 类型 | ❌ | 路径仍主要是字符串 |
| 一等 `Command` 类型 | ❌ | 命令还不是结构化对象 |
| 一等 `ProcessResult` 类型 | ⚠️ | 现在只是 record 返回值，不是稳定 runtime type |
| 一等管道 | ❌ | 还没有真实 `cmd1 |> cmd2` 的进程管道模型 |
| 一等重定向 | ❌ | 还没有 stdin/stdout/stderr 重定向对象 |
| 流式处理 | ❌ | 当前执行模型偏“一次性捕获”，不是流模型 |
| timeout / cancel | ❌ | 长任务控制仍缺失 |
| signal / TTY | ❌ | 服务和交互场景还没法认真承诺 |
| shebang / argv / 脚本入口 | ❌ | 还没有真正替换 `.sh` 入口脚本的语言机制 |
| glob / 文件查询组合子 | ❌ | 自动化脚本里常见，但当前还没有 |

## 当前最该关注的缺口 / Most Important Gaps

### 1. 不是“语法少”，而是“语义不一致”

最危险的不是 parser 不支持，而是：

- parser 支持了
- AST runtime 跑了
- 但 lowering 或 typeck 或 HIR runtime 没闭环

这种状态最容易制造“看起来已经有了”的错觉。

### 2. `?`、方法调用、模式系统是当前的核心风险区

这三块都属于：

- 表面看起来已经有语法
- 实际上还没有完全稳定的统一语义

所以它们必须优先于新语法。

### 3. Bash 替代还没真正开始进入核心难区

现在已经有了“执行命令”的入口，但还没有进入 Bash 真正难替代的部分：

- 管道
- 重定向
- 进程上下文
- 失败组合
- 长任务控制
- 脚本入口模型

## 下一步怎么用这份矩阵 / What To Do Next

这份矩阵对应语言路线图里的 `WP-0A`。
下一步最合理的动作是：

1. 继续把这份矩阵扩成更完整的 feature inventory。
2. 先修矩阵里最红的语义项，而不是继续加新语法。
3. 用这份矩阵反向修正文档里的 `Complete` 表述。
4. 把 `tests/end_to_end.rs` 从占位逻辑替换成真实执行路径。

## 当前结论 / Bottom Line

今天可以对外比较诚实地说：

- Neve 已经是一个**语法表面丰富、核心功能可运行**的语言原型。
- Neve 还不是一个**语义完全收敛的独立完备语言**。
- Neve 已经开始具备**系统脚本能力**，但离**替代 Bash**还差一整层命令与流模型。
