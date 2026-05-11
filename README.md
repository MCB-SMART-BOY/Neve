<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p>A typed language for system automation — and the toolchain that comes with it.</p>

<p>
  <a href="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml">
    <img src="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/MCB-SMART-BOY/Neve/releases">
    <img src="https://img.shields.io/github/v/release/MCB-SMART-BOY/Neve?color=blue">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg">
  </a>
</p>

</div>

---

## 看一眼

Bash 写健康检查：

```bash
#!/bin/bash
set -euo pipefail
PORT=$(grep port /etc/config.toml | cut -d= -f2 | tr -d ' ')
if [ -z "$PORT" ]; then PORT=8080; fi
curl -sf "http://localhost:$PORT/health" || {
    systemctl restart myapp
    sleep 2
    curl -sf "http://localhost:$PORT/health" || { echo "failed" >&2; exit 1; }
}
```

Neve：

```neve
#!/usr/bin/env neve run

let config = io.readFilePath(./config.toml);
let port = config.port or 8080;

io.retry(
    fn() = io.httpGet("http://localhost:{port}/health"),
    maxAttempts = 3,
    backoffMs = 2000,
);
```

类型系统在编译期就告诉你 `config.port` 的类型对不对、`or` 的默认值匹不匹配。不用等到半夜告警响了才发现变量名写错了。

---

## 这语言长什么样

```neve
-- 代数数据类型。编译期检查穷尽性——漏了分支直接报错
enum Health { Alive, Dead(exitCode: Int, stderr: String) }

fn summarize(h: Health) -> String = match h {
    Alive -> "ok",
    Dead(code, stderr) -> "exit {code}: {stderr}",
};

-- 错误传播。? 碰到 None 或 Err 就短路返回，没有 try-catch 地狱
fn loadAndParse(path: Path) -> Result<Config, String> = {
    let raw = io.readFilePath(path)?;
    parseConfig(raw)?
};

-- 安全字段访问。?. 碰到 None 就返回 None，不炸
let port = server?.config?.port or 8080;

-- 惰性求值。只在需要的时候算一次
let expensive = lazy { loadAllFromDisk() };
-- ... 可能根本不用 ...
let value = force(expensive);

-- trait + 泛型
trait HealthCheck {
    fn check(self) -> Health;
}

impl HealthCheck for Server {
    fn check(self) -> Health = {
        let status = io.httpGet("http://localhost:{self.port}/health");
        if status == 200 { Alive } else { Dead(status, "unhealthy") }
    };
}
```

`print` / `println` 是全局的，不用 import。路径字面量 `./foo` 直接就是 `Path` 类型。`" hello {name} "` 字符串插值。

---

## 系统能力

**一等管道**：

```neve
let p = io.command("ls", ["-la"]) |> io.command("grep", ["neve"]);
let result = io.execPipeline(p);
```

命令之间 `|>` 串联，类型是 `Pipeline`。`io.execPipelineStreaming` 逐行处理输出，`io.execPipelineStreamingWithTimeout` 带超时自动 kill。

**流式 I/O + 超时**：

```neve
io.execCommandStreamingWithTimeout(
    io.command("journalctl", ["-f"]),
    fn(line) {
        if line.contains("error") { io.writeFilePath(./errors.log, line) }
    },
    timeoutMs = 5000,  -- 5 秒没新行就 SIGKILL
);
```

**原子写**。先写临时文件再 rename，不会出现写一半断电文件损坏：

```neve
io.atomicWrite(./critical.json, newConfig);
```

**信号处理**：

```neve
io.onSignal(SIGTERM, fn() { io.writeFilePath(./shutdown.log, "graceful") });
io.onSignal(SIGINT, fn() { println("interrupted") });
```

**重试 + 条件等待**：

```neve
io.retry(fn() = io.httpGet("http://..."), maxAttempts = 5, backoffMs = 1000);
io.ensure(fn() = portOpen(8080), timeoutMs = 30000, intervalMs = 500);
```

**二进制数据**。`Bytes` 是一等类型：

```neve
let data = io.readFileBytesPath(./binary.bin);
let hash = bytes.sha256(data);
io.writeFileBytesPath(./copy.bin, data);
```

---

## 反应式

```neve
-- 每 30 秒执行一次
let ticker = io.every(30000);
io.reactive(ticker, fn(_) = io.httpGet("http://.../health"));

-- 文件变更时触发
let watcher = io.watchFile(./config.toml);
io.reactive(watcher, fn(_) { reloadConfig(); println("reloaded") });

-- 信号监听
let sigterm = io.onSignal(SIGTERM);
io.reactive(sigterm, fn(_) { cleanup(); println("shutdown") });
```

`Event<T>` 和 `Live<T>` 是内置的，不需要引入任何库。

---

## 效果系统

```neve
-- 纯函数。不能调用 IO，编译器强制检查
fn add(x: Int, y: Int) -> Int = x + y;

-- 标了 effect 才能做 IO
fn save(path: Path, data: String) effect = io.writeFilePath(path, data);

-- neve check 默认拒绝 effect
-- neve check --pure 连标了 effect 的函数都不让调
```

---

## 工具链

Neve 是一个二进制文件，自带全套工具：

```bash
neve run foo.neve     # 执行脚本（支持 shebang）
neve repl             # 交互式 REPL（历史持久化、Tab 补全、:type 查询、:save、:cd）
neve check foo.neve   # 类型检查
neve fmt file         # 代码格式化
neve doc spec         # 内置文档
neve lsp              # Language Server
```

---

## 形式化验证（Lean 4）

不是说"我们觉得没问题"。`formal/` 目录里有 19 个 Lean 4 模块，核心语义做了机器检查的证明：

- 21 条 EffectEval 规则覆盖全部 I/O 路径（阻塞、流式、超时、retry、ensure、Bytes I/O）
- 全部二元运算符 12/12 有类型安全证明（含除零规则）
- 管道路径遍历防护、环境变量注入防护、缓冲区大小限制——5 项安全审计全部机器验证
- 类型安全定理 `type_safety` + `env_preservation` 引理 + `matchOn` fallthrough 证明

`cd formal && lake build`，一把过。

---

## 装一个

```bash
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh
```

Arch：`paru -S neve-bin`。源码：`cargo install --path neve-cli --locked`。

---

[语言规范](docs/reference/spec.md) · [功能矩阵](docs/project/feature-matrix.md) · [路线图](docs/project/language-roadmap.md) · [更新日志](docs/project/changelog.md) · [贡献指南](docs/contributor/contributing.md)

---

MPL-2.0
