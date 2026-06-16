<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p>一门带类型系统的系统自动化语言 —— 配置、构建、监控、脚本，都在里面。</p>

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

## 一眼看懂

这是 Bash：

```bash
#!/bin/bash
set -euo pipefail
CONFIG="$1"
if [ ! -f "$CONFIG" ]; then echo "没有配置文件" >&2; exit 1; fi
PORT=$(grep port "$CONFIG" | cut -d= -f2 | tr -d ' ')
if [ -z "$PORT" ]; then PORT=8080; fi
for i in 1 2 3; do
    curl -sf "http://localhost:$PORT/health" && break
    sleep 2
done
```

这是 Neve（v4.0）：

```neve
#!/usr/bin/env neve run
use std.io = io

config = io.readFilePath(./config.toml)
port = config.port ?? 8080

io.retry(
    ~io.read(./health-check),
    maxAttempts = 3,
    backoffMs = 2000,
)
```

类型系统在编译期告诉你 `config.port` 对不对、`or` 的默认值类型匹不匹配。不需要等到半夜告警响了才发现变量名拼错了。

---

## 类型系统

不只是"有类型"，而是编译期就能拦住一大类错误。

```neve
-- 代数数据类型。编译期检查穷尽性——漏分支直接报错
type Health { Alive, Dead(exitCode: Int, stderr: String) }

summarize(h: Health) -> String = match h {
    Alive -> "ok",
    Dead(code, stderr) -> "退出码 {code}：{stderr}",
}

-- 错误传播。? 碰到 None 或 Err 就短路返回
loadAndParse(path: Path) -> Result<Config, String> = {
    raw = io.readFilePath(path)?
    parseConfig(raw)?
}

-- 安全字段访问。?. 碰到 None 就返回 None
port = server?.config?.port or 8080

-- 惰性求值。只在第一次 force 的时候计算
expensive = ~(loadAllFromDisk())
-- ……中间可能根本不用 ……
value = force(expensive)

-- trait 和泛型
trait HealthCheck {
    check(self) -> Health
}

impl HealthCheck for Server {
    check(self) -> Health = {
        status = io.read(./health-check)
        if status == "ok" { Alive } else { Dead(1, "不健康") }
    }
}
```

`print` 和 `println` 全局可用，不需要 import。字符串插值 `` `你好 {name}` ``。路径字面量 `./foo` 直接就是 `Path` 类型，不是字符串。支持记录更新 `{ old | field = newValue }`。

---

## 系统能力

**一等管道**。命令之间用 `|>` 串联：

```neve
pipeline = io.command("ls", ["-la"]) |> io.command("grep", ["neve"])
result = io.execPipeline(pipeline)
output = io.processStdout(result)
```

**流式处理**。逐行处理命令输出，带超时自动杀进程：

```neve
io.execCommandStreamingWithTimeout(
    io.command("journalctl", ["-f"]),
    |line| {
        if line.contains("error") {
            io.appendFilePath(./errors.log, line)
        }
    },
    timeoutMs = 5000,
)

-- Stream<T> 变换管道
lines = io.streamLines(./log.txt)
    |> io.streamMap(|l| l.toUpper())
    |> io.streamFilter(|l| l.contains("ERROR"))
    |> io.streamTake(10)
results = io.streamCollect(lines)

-- Stream<T> 管道到命令
io.streamList(["line1", "line2"])
    |> io.streamPipe(io.command("grep", ["line"]))
```

**原子写**。先写临时文件再 rename，不会写出写到一半断电损坏的文件：

```neve
io.atomicWrite(./critical.json, newConfig)
```

**信号处理**。注册操作系统信号回调：

```neve
io.onSignal(SIGTERM, fn() { io.writeFilePath(./shutdown.log, "正常关闭") })
io.onSignal(SIGINT, fn() { println("收到中断信号") })
```

**重试和条件等待**：

```neve
io.retry(~io.read(./health-check), maxAttempts = 5, backoffMs = 1000)
io.ensure(~io.pathExistsPath(./ready), timeoutMs = 30000, intervalMs = 500)
```

**二进制数据**。`Bytes` 是一等类型：

```neve
data = io.readFileBytesPath(./binary.bin)
io.writeFileBytesPath(./copy.bin, data)
```

**文件操作**。增删改查、遍历、权限、符号链接 —— 全有 typed-path 变体：

```neve
io.writeFilePath(./a.txt, "内容")
content = io.readFilePath(./a.txt)
io.appendFilePath(./a.txt, "追加")
io.copyPath(./a.txt, ./b.txt)
io.movePath(./b.txt, ./c.txt)
io.createDirAllPath(./dir/sub)
io.removeDirAllPath(./dir)
io.walk(./dir, |p| { println(p) })
io.chmod(./script, 0o755)
io.symlink(./target, ./link)
io.tempDir(|dir| { io.writeFilePath(./temp.txt, "y"); 42 })
```

---

## 效果系统

```neve
-- 纯函数。不能调 IO，编译器强制检查
add(x: Int, y: Int) -> Int = x + y

-- effect 由编译器自动推断，无需手动标注
save(path: Path, data: String) = io.writeFilePath(path, data)

-- neve check 检测 IO 调用
-- neve check --pure 拒绝所有 effectful 函数调用
```

---

## 工具链

一个二进制文件，自带全套工具：

```bash
neve run foo.neve     # 执行脚本（支持 shebang）
neve repl             # 交互式 REPL（历史持久化、Tab 补全、:type、:save、:cd）
neve check foo.neve   # 类型检查
neve fmt file         # 代码格式化
neve doc spec         # 内置文档
neve lsp              # Language Server
neve search <query>   # 搜索包索引
neve package install <pkg>  # 安装包
neve package remove <pkg>   # 卸载包
neve package list     # 列出已安装包
neve build <pkg>      # 构建包
neve update           # 更新依赖
neve config build     # 构建系统配置
neve config switch    # 切换系统配置
neve store gc         # 垃圾回收
neve registry-update  # 更新 registry 索引
neve registry-serve   # 启动 registry 服务
neve registry-publish # 发布包到 registry
```

---

## 形式化验证

不是说"我们觉得没问题"。`formal/` 目录里有 19 个 Lean 4 模块，核心语义做了机器检查的证明：34 条 EffectEval 规则（v4.3）覆盖了全部 I/O 路径（含 Stream<T> Phase C 5 条规则），全部二元运算符有类型安全证明（含除零规则），管道安全、环境注入防护、缓冲区大小限制共 5 项安全审计全部机器验证。`cd formal && lake build` 一把过。

[语言规范](docs/reference/spec.md) · [功能矩阵](docs/project/feature-matrix.md) · [路线图](docs/project/language-roadmap.md) · [更新日志](docs/project/changelog.md) · [贡献指南](docs/contributor/contributing.md)

---

**Phase 4 (Shell 能力替代) 已完成** ✅ — Stream<T> 14 APIs、E2E 541 测试、Formatter 幂等性 37/37、Clippy 0 warnings。

**Phase 5 (生态补完) 进行中** 🔄 — flake/lock 系统、content-addressed store、registry CLI（17 个命令）、稳定性分级（Tier 1/2/3）。

示例脚本：`examples/test-runner.neve`（测试运行器）、`examples/ci-bootstrap.neve`（CI 启动脚本）、`examples/file-watcher.neve`（文件监控）、`examples/system-config.neve`（系统配置）。

---

MPL-2.0

## 装一个

```bash
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh
```

Arch 用 `paru -S neve-bin`。源码编译：`cargo install --path neve-cli --locked`。
