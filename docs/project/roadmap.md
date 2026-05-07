# Neve 路线图

> 最后更新：v3.3.2

## 已落地

```
v3.3.2 现状

类型系统   ████████░░  fn/impl/lambda 效果闭环、Path 字面量、Record 字段诊断
I/O        ████████░░  流式、原子、管道、重定向
REPL       ██████████  历史、补全、智能输入
事件/反应  ██████░░░░  Event/Live/retry/ensure
包管理     ██████░░░░  构建/锁文件/GC 验证通过，缺注册表
```

---

## 第一阶段：补哲学债（v3.4）

这些不是新功能，是让已经存在的东西真正可靠。

### 效果系统不再自欺欺人

| 任务 | 说明 |
|------|------|
| 跨函数效果追踪 | `effect fn a()` 被纯函数 `b()` 调用时应该报错 |
| 效果不再靠名字匹配 | 从 `io.*` 字符串匹配 → 类型级效果标记 |
| impl 方法默认 pure | 现在 impl 方法没写 effect 也能调 io —— 因为 lowering 默认 `effectful: true` |

### 模式匹配不再放水

| 任务 | 说明 |
|------|------|
| String 穷尽性 | `match "hi" { "x" -> 1 }` 现在是静默通过 |
| Record 穷尽性 | `match {a:1,b:2} { #{a} -> ... }` 不告警 b 未处理 |
| Int/Char 穷尽性 | 基本类型全覆盖 |

### 效果检查去魔法

| 任务 | 说明 |
|------|------|
| 统一效果源 | `is_effectful_builtin` 已去重到 neve-common，但 pure constructor（io.command 等）不应标记为 effectful |
| effect 传播到 trait | trait 定义的方法可以声明 `effect` |

---

## 第二阶段：能力缺口（v3.5-v3.6）

替代 Bash 还差的关键能力。

### 语法

| 任务 | 说明 |
|------|------|
| `|>` 管道语法 | `ls |> grep("neve")` 代替 `io.pipeline([...])` |
| `defer` / `finally` | 资源清理。`defer { cleanup() }` |
| glob | `ls *.neve` 或 `io.glob("*.neve")` |

### 运行时

| 任务 | 说明 |
|------|------|
| 信号处理 | `on Signal::INT { save(); exit(0) }` |
| TTY / 交互输入 | `io.readPassword()`、`io.prompt()` |
| 事件操作符 | `map`/`filter`/`debounce` 对 Event 可用 |
| reactive 块语法 | 不只是 `io.reactive(event)`，而是 `reactive { watch x; y }` 语法 |

### 标准库

| 任务 | 说明 |
|------|------|
| `std.json` | JSON 解析/序列化 |
| `std.net` / HTTP | 比 `fetch.*` 更通用的 HTTP 客户端 |
| `std.regex` | 正则 |

---

## 第三阶段：体验（v3.7+）

### REPL

| 任务 | 说明 |
|------|------|
| 变量名补全 | Tab 补全已定义的变量/函数 |
| 输入验证 | 括号不闭合时提示，不是直接报错 |
| 语法高亮 | rustyline Highlighter |

### 语言

| 任务 | 说明 |
|------|------|
| 自动导入 | `io.println` 不需要 `import std.io` —— io/fmt/path 自动可用 |
| `main` 函数 | 脚本入口，argc/argv 直接注入 |
| 更好的错误信息 | "cannot add" → "String 和 Int 不能相加，你可能想用 `toString(n)`" |

### 工具链

| 任务 | 说明 |
|------|------|
| LSP signatureHelp | 实现或移除声明 |
| 格式化幂等 | `neve fmt` 两次结果一致 |
| `neve test` | 内置测试命令 |

---

## 长期（Phase D/E）

| 任务 | 说明 |
|------|------|
| 二进制缓存 | 包管理 CDN |
| 注册表 | 包索引/搜索 |
| before/after 钩子 | 语言级触发器 |
| transition 状态机 | 编译期状态转换验证 |
| 分布式事件 | 跨机器 Event 传播 |
