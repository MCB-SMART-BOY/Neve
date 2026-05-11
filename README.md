<div align="center">

<img src="assets/logo.svg" width="140" alt="Neve logo">

<h1>Neve</h1>

<p>
  <strong>类型安全</strong> · <strong>管道原生</strong> · <strong>形式化验证</strong>
</p>

<p>A typed language for system automation — configs, builds, monitoring, scripting.</p>

<p>
  <a href="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml">
    <img src="https://github.com/MCB-SMART-BOY/Neve/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/MCB-SMART-BOY/Neve/releases">
    <img src="https://img.shields.io/github/v/release/MCB-SMART-BOY/Neve?color=blue" alt="v3.8.0">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MPL%202.0-brightgreen" alt="MPL-2.0">
  </a>
  <a href="https://aur.archlinux.org/packages/neve-bin">
    <img src="https://img.shields.io/aur/version/neve-bin?color=1793d1&label=AUR" alt="AUR">
  </a>
</p>

<p><strong>Linux</strong> · <strong>macOS</strong> · <strong>Windows</strong></p>

</div>

---

**文档**: [快速入门](docs/user/quickstart.md) · [语言规范](docs/reference/spec.md) · [功能矩阵](docs/project/feature-matrix.md) · [路线图](docs/project/language-roadmap.md) · [效果边界](docs/project/effect-boundary.md) · [更新日志](docs/project/changelog.md)

---

## 这是什么

Neve 是一门带类型系统的系统自动化语言。parser、type checker、evaluator、LSP、formatter、包管理器全部在这个仓库里。

**语言核心**：
- 静态类型 + 类型推导 + trait/impl + 泛型
- 代数数据类型（enum、struct、record、tuple）
- 模式匹配 + 穷尽性检查
- 效果系统：`effect` 关键字标注副作用，`neve check` 默认拒绝 IO

**系统能力**：
- 一等 `Path` / `Command` / `Pipeline` / `Task<T>` / `Bytes` 运行时对象
- `cmd1 |> cmd2 |> cmd3` 管道语法
- 流式 I/O + 超时 + 自动杀进程（`SIGKILL`）
- 文件原子写、两阶段提交
- shebang + argv 脚本入口

**形式化验证**（19 个 Lean 4 模块）：
- EffectEval v4.1：21 条规则覆盖全部 I/O 路径
- BinOp 12/12 类型规则 + 进度证明
- 类型安全定理（`type_safety`）
- 5 项安全审计全部 Lean 验证

---

## 装一个

```bash
# 预编译安装
curl -fsSL https://raw.githubusercontent.com/MCB-SMART-BOY/Neve/master/scripts/install.sh | sh

# Arch Linux
paru -S neve-bin

# 源码安装
git clone https://github.com/MCB-SMART-BOY/Neve.git
cd Neve && cargo install --path neve-cli --locked
```

---

## 跑一下

```neve
-- 路径字面量直接是 Path 类型
let content = io.readFilePath(./Cargo.toml);

-- 管道语法
io.execPipeline(
    io.command("echo", ["hello"]) |> io.command("cat", [])
);

-- 流式处理 + 超时
io.execCommandStreamingWithTimeout(
    io.command("journalctl", ["-f"]),
    fn(line) { println(line) },
    5000
);

-- 效果注解
fn save(path: Path, data: String) effect = io.writeFilePath(path, data);

-- 原子写
io.atomicWrite("/etc/config.toml", newConfig);
```

```bash
neve run script.neve     # 执行脚本
neve repl                # 交互式 REPL
neve check script.neve   # 类型检查
neve fmt file script.neve # 格式化
neve doc spec            # 查看文档
```

---

## 项目结构

```
crates/
├── neve-parser/     解析器
├── neve-hir/        HIR lowering + 名称解析
├── neve-typeck/     类型检查 + 模式分析
├── neve-eval/       HIR 求值器（主路径）+ AST compat
├── neve-std/        标准库（IO、进程、文件）
├── neve-fmt/        代码格式化器
├── neve-lsp/        Language Server
├── neve-frontend/   统一前端入口
├── neve-config/     模块图 + flake 系统
├── neve-common/     共享工具
├── neve-builder/    构建系统
├── neve-store/      内容寻址存储
├── neve-fetch/      依赖获取
formal/              Lean 4 形式化验证（19 模块）
tests/               集成测试（222 E2E）
docs/                文档
examples/            示例代码
```

---

## 开发

```bash
cargo check --workspace          # 编译检查
cargo test --workspace           # 全部测试
cargo test --test end_to_end     # E2E 测试（222 个）
cd formal && lake build          # Lean 验证（19 模块）
```

---

## License

MPL-2.0
