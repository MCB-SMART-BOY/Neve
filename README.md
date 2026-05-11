<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p>A typed language that replaces Bash, Python, and YAML for system automation.</p>

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

Bash 写部署脚本：

```bash
#!/bin/bash
set -euo pipefail
if [ ! -f "$CONFIG" ]; then
    echo "missing config" >&2; exit 1
fi
PORT=$(grep port "$CONFIG" | cut -d= -f2)
if [ -z "$PORT" ]; then PORT=8080; fi
curl -sf "http://localhost:$PORT/health" || {
    systemctl restart myapp; sleep 2
    curl -sf "http://localhost:$PORT/health" || {
        echo "startup failed" >&2; exit 1
    }
}
```

Neve 写同样的东西：

```neve
#!/usr/bin/env neve run
import std.io as io;

let config = io.readFilePath(./config.toml);
let port = config.port or 8080;

io.retry(
    fn() = io.httpGet("http://localhost:{port}/health"),
    maxAttempts = 3,
    backoffMs = 2000,
);
```

类型系统在编译期就告诉你 `config.port` 不存在会怎样、`or` 的默认值类型对不对、`io.httpGet` 返回什么。不用等到半夜脚本挂了才发现拼写错误。

---

## 类型不是负担

```neve
-- 代数数据类型
enum Health { Alive, Dead(exitCode: Int) }

-- 模式匹配 + 穷尽性检查（漏了分支编译不过）
fn describe(h: Health) -> String = match h {
    Alive -> "ok",
    Dead(code) -> "exit {code}",
};

-- 错误传播，? 自动处理 None/Error
fn loadConfig() -> Result<Config, String> = {
    let raw = io.readFilePath(./config.toml)?;   -- IO 可能失败
    parseConfig(raw)?                              -- 解析可能失败
};

-- trait + 泛型
trait HealthCheck {
    fn check(self) -> Health;
}
```

没有 `null`，没有 `undefined`，没有 `try-catch` 的缩进地狱。

---

## Bash 做不到的事

**流式管道 + 超时**。`journalctl -f` 的输出逐行处理，超过 5 秒没新行就自动 kill：

```neve
io.execCommandStreamingWithTimeout(
    io.command("journalctl", ["-f"]),
    fn(line) {
        if line.contains("error") {
            io.writeFilePath(./errors.log, line);
        }
    },
    timeoutMs = 5000,
);
```

**一等管道**。命令之间用 `|>` 串联，类型是 `Pipeline`：

```neve
let p = io.command("ls", ["-la"]) |> io.command("grep", ["neve"]);
let result = io.execPipeline(p);
let lines = io.processStdout(result);
```

**原子写**。不会出现写到一半断电文件损坏：

```neve
io.atomicWrite(./critical.json, newConfig);
io.atomicWriteAll(./dir/, [("./a.txt", contentA), ("./b.txt", contentB)]);
```

---

## 装一个

```bash
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh
```

或者 Arch：`paru -S neve-bin`。源码：`cargo install --path neve-cli --locked`。

然后：

```bash
neve repl          # 交互式，有 Tab 补全
neve run foo.neve  # 跑脚本
neve check foo.neve # 类型检查
```

---

## 形式化验证

核心语义用 Lean 4 做了机器检查的证明。不是"我们觉得没问题"，是编译器级别的保证：

- 效果系统 21 条规则覆盖全部 I/O 路径
- 全部二元运算符 12/12 有类型安全证明
- 管道安全、环境注入防护、缓冲区溢出防护——5 项安全审计全部机器验证

`formal/` 目录，`lake build` 一把过。

---

## 更多

[语言规范](docs/reference/spec.md) · [功能矩阵](docs/project/feature-matrix.md) · [路线图](docs/project/language-roadmap.md) · [更新日志](docs/project/changelog.md)

---

MPL-2.0
