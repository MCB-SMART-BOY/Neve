# Neve: Better Bash + Better Nix — Roadmap

## 当前状态

Neve v4.0.0 已经具备系统脚本和构建系统的基础能力：

| 能力 | Bash | Neve | Nix |
|------|------|------|-----|
| 文件读写 | ✅ | ✅ | ✅ |
| 进程执行 | ✅ | ✅ | ⚠️ |
| 管道 | ✅ | ✅ | ❌ |
| 流式处理 | ❌ | ✅ | ❌ |
| 超时/kill | ⚠️ | ✅ | ❌ |
| 信号处理 | ⚠️ | ✅ | ❌ |
| 后台任务 | ✅ | ✅ | ❌ |
| 沙箱构建 | ❌ | ✅ | ✅ |
| 内容寻址 | ❌ | ✅ | ✅ |
| 可复现 | ❌ | ✅ | ✅ |
| GC | ❌ | ✅ | ✅ |
| Lockfile | ❌ | ✅ | ✅ |
| 类型系统 | ❌ | ✅ | ❌ |
| Effect 边界 | ❌ | ⚠️ | ❌ |

## 路线图

### Phase 1: Effect 类型系统 v2（基础）
**目标：副作用可审计、可组合**

- [ ] `Effect` 类型加入函数类型：`Fn(Vec<Ty>, Box<Ty>, Effect)`
- [ ] Effect 多态：`fn map<A, B>(f: fn(A) -> B, xs: List<A>) -> List<B>` 自动传播 effect
- [x] Effect 推断：pure 函数不写 `effect` 也自动检测（v4.0 已实现）
- [ ] `neve check` 默认启用（不再需要 `--pure` flag）
- [ ] Effect 诊断改进：显示 effect 传播链（谁调了谁导致 effect）

**受益：** 脚本中的副作用调用链清晰可见；构建系统中的纯函数得到编译器保证。

---

### Phase 2: 脚本体验升格（Bash 替代）
**目标：日常脚本从 Bash 迁移到 Neve**

- [ ] `io.shell` 安全模式：`io.shell("...", strict=true)` 禁止拼接
- [ ] 命令插值语法：`io.cmd\`echo ${name}\`` 类型安全的命令构造
- [x] `io.chmod` / `io.chown` / `io.symlink` / `io.readlink` ✅
- [x] `io.getEnv` / `io.setEnv` / `io.unsetEnv` / `io.env()` ✅
- [x] `io.tempDir(fn)` — 临时目录 + 自动清理
- [x] `io.walk(dir)` — 递归遍历目录
- [x] `io.watch(dir)` — 已有 `io.watchFile` + `io.eventNext` 事件系统覆盖，无需额外实现
- [ ] 更好的错误信息：进程失败时显示 stdout/stderr 摘要
- [x] `io.args()` 结构化解析：返回 `(List<String>, Record)` 元组；`-v`→Bool、`-j8`→Int、`-f out`→String、`-10`→位置参数、`--` 分隔
- [x] 内置 CLI 参数解析：`io.args()` 本身即解析器，无需额外 API
  ```neve
  let (files, { v, j = 4 }) = io.args();  // 解构即解析
  ```

**受益：** Neve 脚本比 Bash 更安全、更可读、更可维护。

---

### Phase 3: 构建系统升格（Nix 替代）
**目标：用 Neve 描述和构建软件包**

- [ ] Derivation 一等语法：`derivation { ... }` 作为语言内置
- [ ] 并行构建调度（`io.buildParallel(derivations)`）
- [ ] 二进制缓存（`io.cache.push` / `io.cache.pull`）
- [ ] 跨平台编译支持（`target = "aarch64-linux"`）
- [ ] Flake 输出 schema 验证（类型检查 flake outputs）
- [ ] `neve build` 增量构建（只重建变更的 derivation）
- [ ] `neve shell` 开发环境（自动加载依赖）
- [ ] `neve run` 应用执行（类似 `nix run`）

**受益：** 比 Nix 更简洁的语法，完整的类型安全和 effect 审计。

---

### Phase 4: 生态与分发
**目标：让 Neve 可以被广泛使用**

- [ ] `neve-neve` 包仓库（类似 npm/cargo）
- [ ] `neve init` 项目脚手架
- [ ] LSP 完善：代码补全、重构、格式化-on-save
- [ ] VS Code 插件
- [ ] 多平台二进制分发（GitHub Releases）
- [ ] Homebrew / AUR / nixpkgs 更新
- [ ] 文档：Neve by Example、Cookbook

---

## 优先级建议

```
Phase 1 (completed: v3.6-v3.12)
  └─ Effect 类型系统 v2
     耗时：2-3 轮

Phase 2 (completed: v3.12-v3.17)
  └─ 脚本体验：chmod/symlink/tempDir/walk/参数解析
     耗时：3-4 轮

Phase 3 ✅ (completed: v3.19-v4.0)
  └─ 构建系统：derivation 语法、并行、缓存
     耗时：4-5 轮

Phase 4 (planned: v4.1+)
  └─ 生态：包仓库、插件、文档
     持续
```
