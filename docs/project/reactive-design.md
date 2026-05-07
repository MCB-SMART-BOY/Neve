# Neve 反应式与时序约束设计

> 状态：**草案 v0**  
> 目标：让 Neve 原生支持以下数据库级能力——触发器（before/after/on）、反应式计算（reactive）、时序约束（ensure/always）、状态机（transition）、守卫（guard/invariant）、级联传播（cascade/invalidate）。用于系统监控、配置自动化和自愈场景。

---

## 1. 设计目标

### 核心原则

1. **安全优先**：所有反应式行为必须经过效果系统（effect system）显式标记，不可静默发生
2. **可组合**：触发器、约束、反应式绑定应该像普通值一样可组合
3. **可取消**：所有异步/反应式操作必须有明确的取消路径
4. **可观测**：运行时状态必须可查询、可调试
5. **渐进式**：从最小可用子集开始，逐步扩展

### 不做什么

- 不做通用 FRP（Functional Reactive Programming）框架
- 不做分布式事件总线
- 不做实时硬保证（hard real-time）

---

## 2. 架构总览

```
Layer 4: 状态转换约束 (State Transition Constraints)
  transition, guard, invariant, cascade, invalidate
         ↑ 通常与时序约束组合使用
Layer 3: 时序约束 (Temporal Constraints)  
  ensure, require, within, retry, backoff, always, eventually, leadsTo
         ↓ 建立在
Layer 2: 反应式绑定与钩子 (Reactive Bindings & Hooks)
  reactive, before, after, around, on, watch
         ↓ 建立在
Layer 1: 事件基础设施 (Event Infrastructure)
  Event<T>, emit, channel, watchFile, watchProcess
```

| 概念 | 数据库类比 | Neve 语法 | 类型安全 | 效果标记 |
|------|-----------|----------|---------|---------|
| 事件流 | 无直接类比 | `Event<T>` | ✅ 泛型 | consumer 标记 effect |
| 前置钩子 | `BEFORE INSERT/UPDATE/DELETE` | `before <target> effect = fn(x) -> Result<x, Err>` | ✅ 输入输出类型检查 | ✅ effect |
| 后置钩子 | `AFTER INSERT/UPDATE/DELETE` | `after <target> effect = { ... }` | ✅ 无返回值 | ✅ effect |
| 包围钩子 | `INSTEAD OF` | `around <target> effect = fn(inner, x) { ... }` | ✅ inner 函数类型检查 | ✅ effect |
| 事件响应 | `ON EVENT` | `on <Event> { ... }` | ✅ Event<T> 类型匹配 | ✅ effect |
| 守卫 | `CHECK` 约束 | `guard <condition> { require ... }` | ✅ Bool 条件 | ✅ effect |
| 不变式 | `CHECK` / `NOT NULL` | `invariant <condition>` | ✅ 编译期/运行时双重检查 | 无需标记 |
| 状态转换 | 状态机约束 | `transition <Type> { State -> State ... }` | ✅ 穷尽性检查 | 无需标记 |
| 级联 | `ON DELETE CASCADE` | `cascade <action>() { ... }` | ✅ 签名匹配 | ✅ effect |
| 失效传播 | 物化视图刷新 | `invalidate <memo>` | ✅ 依赖图 | ✅ effect |
| 反应式绑定 | 物化视图 | `reactive { watch x; ... }` | ✅ 块类型推断 | ✅ effect |
| 时序约束 | 无直接类比 | `ensure <cond> within <T>` | ✅ 条件类型检查 | ✅ effect |
| 重试 | 无直接类比 | `retry(fn, backoff=exponential(...))` | ✅ 泛型 | ✅ effect |
|------|-----------|----------|
| 事件流 | N/A（底层原语） | `Event<T>` |
| 触发器 | `CREATE TRIGGER BEFORE/AFTER` | `before`/`after` 块 |
| 守卫 | `CHECK` 约束 | `guard` 块 |
| 级联 | `ON DELETE CASCADE` | `cascade` 声明 |
| 状态转换 | 状态机约束 | `transition` 类型 |
| 时序约束 | 无直接类比 | `ensure`/`require`/`always` |

---

## 3. Layer 1 — 事件基础设施

### 3.1 `Event<T>` 类型

```neve
-- Event<T> 表示一个类型为 T 的事件流
-- 语义：可被监听、过滤、转换

type Event<T>;
```

**关键设计决策**：

- `Event<T>` 是**一等类型**，有 runtime identity（类似 `Task<T>`）
- 事件流是**拉取模型**（pull-based），不是推送模型——避免回调地狱
- 每次 `.next()` 返回 `Option<T>`（流结束返回 None）

### 3.2 事件源（Event Sources）

```neve
-- 文件变化事件
watchFile(path: Path) -> Event<FileChange> effect

-- 进程退出事件
watchProcess(cmd: Command) -> Event<ProcessResult> effect

-- 定时器事件
every(interval: Duration) -> Event<Instant> effect

-- HTTP 健康检查事件
healthCheck(url: String, interval: Duration) -> Event<HealthStatus> effect

-- 自定义事件
channel<T>() -> (Sender<T>, Receiver<T>)
  -- Sender<T>: emit(value: T) -> Unit
  -- Receiver<T>: impl Event<T>
```

### 3.3 事件操作符

```neve
-- 过滤
filter(predicate: T -> Bool) -> Event<T>

-- 映射
map<U>(f: T -> U) -> Event<U>

-- 合并（任意一个触发）
merge(other: Event<T>) -> Event<T>

-- 采样（每 N 个取一个）
sample(n: Int) -> Event<T>

-- 去重（连续相同值跳过）
distinct() -> Event<T>

-- 去抖动（在安静期内不触发）
debounce(window: Duration) -> Event<T>
```

### 3.4 类型与效果

```neve
-- Event 操作符是纯函数
filter : Event<T> -> (T -> Bool) -> Event<T>    -- pure!
map    : Event<T> -> (T -> U) -> Event<U>       -- pure!

-- 事件源是 effectful（需要系统资源）
watchFile : Path -> Event<FileChange> effect     -- 需要 inotify/fsevent
every     : Duration -> Event<Instant> effect     -- 需要定时器

-- 消费事件是 effectful
next      : Event<T> -> Option<T> effect          -- 阻塞等待
```

**安全保证**：事件源和消费标记为 `effect`，变换操作保持 pure。效果系统确保你不会在纯函数中意外触发副作用。

---

## 4. Layer 2 — 反应式绑定

### 4.1 `reactive` 表达式

```neve
-- reactive 块：当依赖的事件触发时，自动重新求值
let status = reactive {
    let cpu = watch every(5.seconds);
    let mem = watch every(5.seconds).map(fn(_) { readMemUsage() });
    if cpu > 90.0 || mem > 95.0 {
        Alert("resource exhausted")
    } else {
        Healthy
    }
} effect;
```

**语义**：
- `reactive { ... } effect` 创建一个反应式块
- 块内的 `watch*` 调用被自动追踪为依赖
- 任何依赖触发时，块重新求值
- `reactive` 自身是 `effect`（需要持续运行）

### 4.2 依赖追踪

```neve
-- 显式依赖声明
let cpuAlert = reactive {
    let cpu = watch every(5.seconds).map(fn(_) { readCpuUsage() });
    cpu > 90.0
} effect;
```

**设计权衡**：选择**显式 watch** 而非自动依赖追踪（如 Vue/Solid 的魔法 getter）。原因：
1. 显式 = 可审计，符合 Neve 的"无魔法"哲学
2. 效果系统可以精确标记哪些操作是 effectful
3. 编译期可知依赖图，便于死循环检测

### 4.3 生命周期管理

```neve
-- reactive 块返回 Live<T>，可以取消
let live: Live<Status> = reactive { ... } effect;

-- Live<T> 提供：
live.current() -> T              -- 获取当前值（不阻塞）
live.awaitNext() -> T effect     -- 等待下一个值
live.cancel() -> Unit effect     -- 取消反应式块
```

---

## 5. Layer 3 — 时序约束

### 5.1 `ensure` 表达式

```neve
-- "确保条件在指定时间内成立"
ensure cpu < 90.0 within 5.minutes effect
    onViolation { alert("CPU too high") }
    onTimeout   { alert("check timed out") };
```

### 5.2 `require` 表达式

```neve
-- "此条件必须始终保持"
require service.running effect
    retry 3 times backoff 30.seconds
    onPermanentFailure { notify(onCall) };
```

### 5.3 `retry` / `backoff` 组合子

```neve
-- 重试策略
retry(
    fn() { http.get("https://api.example.com/health") },
    maxAttempts = 5,
    backoff = exponential(100.millis, 2.0, 30.seconds)
) effect;

-- 内置策略
exponential(initial: Duration, factor: Float, max: Duration) -> Backoff
linear(step: Duration, max: Duration) -> Backoff
fixed(interval: Duration) -> Backoff
```

### 5.4 时序逻辑运算符

```neve
-- always: 条件必须始终成立
always(check) within 1.hour

-- eventually: 条件最终必须成立
eventually(service.healthy) within 5.minutes

-- leadsTo: A 成立后 B 必须在 T 内成立
restart leadsTo healthy within 30.seconds
```

---

## 6. Layer 2-B — 钩子（Hooks）

钩子拦截特定操作，在执行前后插入逻辑。这是数据库 `BEFORE/AFTER/INSTEAD OF` 触发器的直接对应。

### 6.1 `before` / `after` 钩子

```neve
-- before: 在执行操作前运行，可以阻止操作
before write ./config.toml effect = fn(content: String) -> Result<String, String> {
    if !validateToml(content) {
        Err("invalid TOML")          -- 返回 Err 阻止写入
    } else {
        Ok(content)                  -- 返回 Ok 允许，可修改内容
    }
};

-- after: 在操作成功后运行，不能阻止操作
after write ./config.toml effect = {
    io.println("config updated, reloading...");
    reloadService();
};

-- 钩子可以绑定到多种目标
before exec(cmd: Command) effect = fn(cmd: Command) -> Command {
    io.println("about to execute: {}", cmd.program);
    cmd
};
```

### 6.2 `around` 钩子（包围通知）

```neve
-- around: 完全包围一个操作，可替换其行为
around write ./config.toml effect = fn(inner: fn(String) -> Unit, content: String) -> Unit {
    let backup = io.readFilePath(./config.toml);
    inner(content);                    -- 调用原始操作
    if !service.healthy() {
        io.writeFilePath(./config.toml, backup);  -- 回滚
    }
};
```

### 6.3 `on` 事件钩子

```neve
-- on: 响应特定事件（不拦截）
on Crash { restart(); }
on ConfigChange { reload(); }
on Timeout(5.seconds) { alert(); }

-- 相当于 after 的简化语法
```

### 6.4 钩子的作用域与取消

```neve
-- 钩子绑定到当前作用域的生命周期
scope {
    before write ./config.toml effect = validate;
    -- 验证在此作用域内有效
};
-- 离开作用域后钩子自动移除

-- 或显式管理
let hook = before write ./config.toml effect = validate;
hook.cancel();  -- 显式取消
```

---

## 7. Layer 4 — 状态转换约束

这是数据库 `CHECK` 约束、`TRIGGER ... FOR EACH ROW` 和状态机验证的对应。

### 7.1 `guard` — 不变式

```neve
-- guard: 一个必须始终保持的条件
guard service.running {
    require checkHealth() within 30.seconds
        onViolation { restart() };
};

-- 等价于：任何可能违反条件的地方都要检查
```

### 7.2 `invariant` — 静态约束

```neve
-- invariant: 声明式约束，编译器尽可能静态验证
invariant port > 0 && port < 65536;
invariant config.timeout >= 0;
invariant len(allowedHosts) > 0;

-- 违反时编译警告或运行时 panic
```

### 7.3 `transition` — 状态机

```neve
-- transition: 声明有效状态及其转换
type ServiceState = enum { Stopped, Starting, Running, Stopping, Failed };

transition ServiceState {
    -- 合法的转换路径
    Stopped  -> Starting;
    Starting -> Running;
    Starting -> Failed    if timeout 30.seconds;
    Running  -> Stopping;
    Stopping -> Stopped;
    Stopping -> Failed    if timeout 10.seconds;
    _        -> Failed;   -- 任何状态都可以直接进入 Failed

    -- 非法转换在编译期或运行时报错
};

-- 使用时
let state: ServiceState = Stopped;
state.transition(Starting);    -- ✓ 合法
state.transition(Running);     -- ✗ 编译错误：Stopped 不能直接到 Running
```

### 7.4 `cascade` — 级联操作

```neve
-- cascade: 当主对象变化时自动传播
cascade service.stop() {
    notify(webhook, "stopping");
    drainConnections(30.seconds);
    closeSockets();
};

cascade config.reload() {
    invalidate cache;
    reloadTemplates();
    notify(webhook, "reloaded");
};

-- 等价于：
after exec service.stop effect = {
    notify(webhook, "stopping");
    drainConnections(30.seconds);
    closeSockets();
};
```

### 7.5 `invalidate` — 失效传播

```neve
-- invalidate: 标记派生数据为脏，触发重新计算
let cache = memoize { expensiveQuery() };
let derived = cache.map(fn(data) { transform(data) });

invalidate cache;  -- 下次访问 cache 或 derived 时重新计算
```

---

## 8. 安全模型

### 6.1 效果层级

```
pure     : 计算，无副作用，无事件
event    : 事件变换（map/filter/merge），无 I/O
io       : 文件/网络/进程 I/O
reactive : 持续运行，消耗系统资源
```

```neve
fn transform(e: Event<Int>) -> Event<String> =       -- pure
    e.map(fn(x) { toString(x) });

fn watch() -> Event<Int> effect =                    -- io
    every(1.second).map(fn(_) { readCpuUsage() });
```

### 6.2 资源泄漏防护

```neve
-- Live<T> 必须显式取消或由所有者 scope 管理
scope {
    let live = reactive { ... } effect;
    -- live 在 scope 结束时自动取消
};
```

### 6.3 死循环检测（编译期）

```neve
-- 这会被编译器拒绝：reactive 块不能同步修改自己的依赖
let x = reactive {
    x.current() + 1   -- 编译错误：循环依赖
} effect;
```

### 6.4 并发安全

```neve
-- Event 操作符是单线程模型
-- 多个 reactive 块可以通过 Task<T> 并发运行
let a = reactive { watch fileA } effect;
let b = reactive { watch fileB } effect;
let combined = Task.all([a, b]);
```

---

## 9. 实现路线图

### Phase 1 — 事件基础设施（v3.4）

```
Event<T> 类型              2-3 天
watchFile / every          1-2 天
map / filter / merge        1 天
Sender<T> / Receiver<T>    1 天
```

### Phase 2 — 反应式绑定（v3.5）

```
reactive 块                 2-3 天
Live<T> 生命周期            1-2 天
依赖追踪（显式 watch）       1 天
```

### Phase 3 — 时序约束（v3.6）

```
ensure / require             2 天
retry / backoff              1 天
时序逻辑运算符                1-2 天
```

---

## 10. 使用案例

### 8.1 自动重启崩溃服务

```neve
watchProcess(io.command("myapp", []))
    .filter(fn(r) { !r.success })
    .onTrigger(fn(_) {
        io.println("restarting...");
        io.execCommand(io.command("myapp", []));
    }) effect;
```

### 8.2 配置文件热重载

```neve
watchFile(./config.toml)
    .debounce(500.millis)
    .onTrigger(fn(_) {
        let config = io.readFilePath(./config.toml);
        applyConfig(config);
    }) effect;
```

### 8.3 健康检查 + 告警

```neve
ensure http.get("https://api.example.com/health").status == 200
    within 30.seconds
    retry 3 times backoff 5.seconds
    onViolation { notifySlack("API down") }
    onTimeout   { notifySlack("health check stuck") }
    effect;
```
