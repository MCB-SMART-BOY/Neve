# Neve 设计审查报告 (Design Audit)

**日期**: 2026-06-17 | **版本**: v4.0.1 | **审查范围**: 全项目 (17 crates, 541 E2E)

---

## 概述

从成熟语言设计师角度，对 Neve 项目进行全方位审查。共发现 **62 个问题**，按严重程度分为 Critical / High / Medium / Low。

### 问题分布

| 类别 | Critical | High | Medium | Low | 合计 |
|------|----------|------|--------|-----|------|
| 架构与依赖 | 3 | 2 | 4 | 3 | 12 |
| 安全与错误处理 | 1 | 3 | 2 | 0 | 6 |
| 类型系统正确性 | 1 | 4 | 7 | 7 | 19 |
| 测试与 CI | 0 | 6 | 4 | 4 | 14 |
| API 与文档 | 7 | 0 | 2 | 2 | 11 |
| CLI/REPL/LSP | 3 | 4 | 4 | 5 | 16 |

---

## Critical (必须立即修复)

### C1. reqwest TLS 特性被覆盖 — HTTPS 请求可能失败

**文件**: `crates/neve-fetch/Cargo.toml:12`, `neve-cli/Cargo.toml:44`

workspace 定义 `reqwest = { features = ["blocking", "rustls-tls"] }`，但两个 crate 用 `features = ["blocking"]` 覆盖了 workspace 的 features，导致 `rustls-tls` 丢失。这意味着所有 HTTPS 请求(registry、fetch)都会因为没有 TLS 后端而失败。

```toml
# 当前(错误):
reqwest = { workspace = true, features = ["blocking"] }
# 应为:
reqwest = { workspace = true }
```

**影响**: neve-fetch 的 URL 获取、neve-cli 的 registry 操作全部无法使用 HTTPS。

### C2. AST 节点完全未密封 — 添加变体是破坏性变更

**文件**: `crates/neve-syntax/src/ast.rs`, `expr.rs`, `types.rs`

所有 AST 类型 (`ExprKind` 34 variants, `ItemKind` 11 variants, `TypeKind`, `PatternKind`, `BinOp`, `UnaryOp`, `VariantKind`, `StmtKind`) 都缺少 `#[non_exhaustive]`，且所有字段都是 `pub`。任何外部 crate 可以绕过 Parser 直接构造 AST 节点。

```rust
// 当前: 添加 variant = breaking change
pub enum ExprKind { ... }
// 应为:
#[non_exhaustive]
pub enum ExprKind { ... }
```

**影响**: 语义版本控制形同虚设。每次添加语法特性都需要 major version bump。

### C3. 大量 `pub` 类型应改为 `pub(crate)`

以下类型标记为 `pub` 但从未被 crate 外部使用，泄露了实现细节：

| 类型 | 位置 | 建议 |
|------|------|------|
| `InferContext` | `typeck/src/infer.rs:12` | `pub(crate)` |
| `Substitution` | `typeck/src/unify.rs:14` | `pub(crate)` |
| `ThunkState` | `eval/src/value.rs:40` | `pub(crate)` |
| `PatternClass` | `eval/src/pattern.rs:125` | `pub(crate)` |
| `Discriminant` | `eval/src/pattern.rs:179` | `pub(crate)` |
| `LiteralValue` | `eval/src/pattern.rs:228` | `pub(crate)` |
| `MatchHints` | `eval/src/pattern.rs:255` | `pub(crate)` |
| `ModuleCache` | `hir/src/incremental.rs:15` | `pub(crate)` |
| `format_value` | `eval/src/builtin.rs:1847` | `pub(crate)` |
| `LinesChannelRx/BytesChannelRx` | `eval/src/value.rs:908-911` | `pub(crate)` |

### C4. README 语法与解析器不一致

**文件**: `README.md`

README 中多处使用已废弃的语法，与实际 parser 行为不匹配：

| README 内容 | 实际 v4.0 语法 |
|-------------|---------------|
| `lazy { loadAllFromDisk() }` | `~(loadAllFromDisk())` |
| `save(...) effect = ...` | `save(...) = ...` (effect 自动推导) |
| 多处的 `then` 关键字 | `->` |
| `\|\| io.read(...)` 双竖线闭包 | `\|x\|` 单竖线 (不支持零参 `\|\|`) |

### C5. 所有示例文件使用过时语法

**文件**: `examples/` 目录下所有 `.neve` 文件

- 10 个文件中 8 个使用 `import` (legacy) 而不是 `use` (canonical)
- 全部使用 `then` 而不是 `->`
- 部分使用 `as` 别名语法而非 `=`
- 部分使用 `{ }` 记录语法而非当前语法

`learning/` 目录下的文件语法不统一：04 用 `use`，05 用 `then`，06 用 `import`。

### C6. Formatter 静默丢弃所有注释

**文件**: `crates/neve-fmt/src/format.rs`

AST 不包含注释信息(CST 的常见做法)，但 formatter 没有保留注释的策略。格式化后的代码会丢失所有 `&` 和 `--` 注释。`test_comment_preserved` 测试是假阳性 — 比较发生在注释已被 parse 丢弃之后。

### C7. `CacheStats` 命名冲突

**文件**: `hir/src/incremental.rs:98` vs `store/src/cache.rs:1505`

两个完全不同的 `CacheStats` struct 命名为同名，都是 `pub`。同时依赖 `neve-hir` 和 `neve-store` 的代码会产生歧义。

---

## High (应尽快修复)

### H1. Phase D 残留: neve-eval 依赖未使用的 neve-parser

**文件**: `crates/neve-eval/Cargo.toml:13`

`ast_eval.rs` 已删除但 `neve-parser.workspace = true` 仍在。edition 2024 对 lib target 的 `unused_crate_dependencies` 是 **deny-by-default**。

### H2. neve-config 有 3 个未使用的依赖

**文件**: `crates/neve-config/Cargo.toml:10,13,15`

`neve-common`、`neve-lexer`、`neve-parser` 在 `neve-config/src/` 中没有被引用。同样是 edition 2024 的潜在编译错误。

### H3. 泛型 trait bound 从未在调用点强制执行

**文件**: `typeck/src/check/mod.rs:2718`, `typeck/src/traits.rs:655-718`

`ConstraintSolver` 结构体存在但**从未被实例化**。当多态函数 `fn foo[T: Show](x: T) = ...` 被调用时，实际类型参数不检查是否满足 `Show` trait。trait bound 只检查关联类型(impl 层面)，函数调用点的泛型约束完全缺失。

### H4. `types_match` 忽略类型参数

**文件**: `typeck/src/traits.rs:361-383`

`List[Int]` 和 `List[String]` 在 trait impl 查找时被视为相同类型:
```rust
(TyKind::Named(id1, _), TyKind::Named(id2, _)) => id1 == id2,
```
如果存在 `impl Show for List[Int]` 和 `impl Show for List[String]`，会选错 impl。

### H5. Enum 类型构造函数始终有空类型参数

**文件**: `typeck/src/check/mod.rs:1997,2023,2045,3439`

所有 enum 类型使用 `TyKind::Named(def_id, Vec::new())`(空泛型参数)。泛型 enum (如 `enum Result[A, E] { Ok(A), Err(E) }`) 的类型参数在类型检查期间被丢弃，导致 `Ok(42)` 和 `Ok("hello")` 可能被统一为相同的 enum 类型。

### H6. 42 个 Mutex 中毒风险点

**文件**: `eval/src/eval.rs`(5), `std/src/io/mod.rs`(30), `store/src/cache.rs`(7)

所有 `.lock().unwrap()` 调用在 mutex 中毒时会 panic 而不是恢复。spawn registry (`io/mod.rs` 中的 30 个) 风险最高 — 进程执行期间的线程 panic 会导致后续所有 spawn 操作失败。

**修复方案**: `lock().unwrap_or_else(|e| e.into_inner())` 或使用 `parking_lot::Mutex`(不中毒)。

### H7. `fork()` 在多线程环境中的安全性

**文件**: `builder/src/sandbox.rs:665`

Rust 程序中 `fork()` 可能死锁 — 如果 fork 时另一个线程持有锁，子进程继承锁的状态。sandbox 在 fork 后立即 exec，这减轻了风险但不能完全消除。需要审计哪些锁在 fork 时刻可能被持有。

### H8. 无递归深度限制

**文件**: `eval/src/eval.rs`

TCO 处理了尾递归，但非尾递归函数(如 `fn sum(n) = if n == 0 -> 0 else n + sum(n - 1)`)没有深度计数器，中等输入就会栈溢出。

### H9. occurs check 在动态/安全记录字段中被绕过

**文件**: `typeck/src/check/mod.rs:1459-1560`

`constrain_dynamic_record_field`、`constrain_safe_record_base`、`extend_safe_record_base` 直接调用 `self.subst.extend(var, new_ty)`，绕过了 `unify()` 中的 occurs check。可能产生无限类型而没有任何诊断。

### H10. LSP hover 中类型名显示为 "T"

**文件**: `lsp/src/backend.rs` `def_id_to_name_hint`

用户自定义类型的 hover/inlay hint 全部显示 `"T"` 而非实际类型名。`def_id_to_name_hint` 返回硬编码的 `"T".to_string()`。

### H11. Formatter 不在超长行换行

**文件**: `fmt/src/format.rs`

`would_exceed_width` 检查只对 Record 和 List 进行。函数调用、二元表达式、match 表达式超过 `max_width=100` 时不会换行，直接产生超长输出。

### H12. `path.to_str().unwrap()` 在非 UTF-8 路径上 panic

**文件**: `cli/src/commands/fmt.rs:120`

Linux 路径是任意字节序列。非 UTF-8 路径会直接 panic CLI。

### H13. Ctrl+C 在 REPL 求值期间无保护

**文件**: `cli/src/commands/repl.rs:262-287`

`evaluate_repl_input` 同步执行期间按 Ctrl+C 会直接杀死进程(无历史保存)。应注册 SIGINT handler 或在求值循环中检查信号。

---

## Medium (应该修复)

### M1. neve-derive 依赖未使用的 neve-common
`crates/neve-derive/Cargo.toml:10` — edition 2024 潜在编译警告/错误。

### M2. neve-store 依赖未使用的 neve-common
`crates/neve-store/Cargo.toml:10` — 同上。

### M3. neve-fetch 重复声明依赖而非使用 workspace ref
`tar`、`flate2`、`xz2`、`tempfile`、`blake3`(未使用)在 `neve-fetch/Cargo.toml` 中直接写版本号。

### M4. neve-std 和 neve-cli 有未跟踪的孤立依赖
`glob`、`rpassword`、`termimad` 不在 workspace `[dependencies]` 中，版本漂移风险。

### M5. neve-builder nix 特性重复声明
直接写了完整的 `nix = { version = "0.29", features = [...] }` 而非 `nix = { workspace = true }`。

### M6. `instantiate` 中的 Forall 参数绑定依赖脆弱的名称匹配
`unify.rs:380-401` — 通过 `"t0"` 前缀和 `parse::<u32>()` 解析参数名。

### M7. SafeRecordBase/DynamicRecord 在穷举性分析中未处理
`pattern_analysis.rs:627-630` — match 穷举性检查不覆盖安全记录访问和动态记录。

### M8. Thunk 在二进制/比较操作中未 force
`eval.rs:913-1065` — 如果 Thunk 直接到达算术操作会得到奇怪的报错而非自动求值。

### M9. 信号检查只在 TCO 循环中
`eval.rs:1126` — `map`/`fold`/`filter` 等长时间运行的内置函数不检查信号，求值器不可中断。

### M10. 同一作用域内无重复名称检测
`hir/src/resolve.rs:755-761` — `define_local` 静默覆盖已存在的绑定。

### M11. 名称解析未找到时的错误路径不一致
未解析的简单名称 → `Builtin(name)`，未解析的路径 → `Global(DefId(u32::MAX))` — 用户得到不同质量的错误信息。

### M12. top-level let 总是标记为 effectful
`hir/src/resolve.rs:1061-1083` — `effectful: true` 硬编码，意味着模块级 let 绑定总是允许副作用。

### M13. eval 和 typeck 各自独立收集 VariantCtor 信息
`eval.rs:374-384` vs `check/mod.rs:2028-2053` — 没有共享的事实来源，可能产生不一致。

### M14. `:cd` 改变全局 CWD 但不更新语义状态
`repl.rs:235` — REPL 中 `:cd` 后模块解析可能用错误路径。

### M15. LSP `did_change` 无防抖
每次按键触发完整的重新分析和诊断发布，大文件性能差。

### M16. CLI 错误消息丢弃诊断细节
`eval.rs:20` — `return Err("parse error")` 不报告错误数量/位置/内容。

### M17. 所有错误使用 exit code 1
无区分：parse error / type error / runtime error / IO error 全返回 1。

### M18. `debug_assert!` 用于缩进平衡检查
`fmt/src/format.rs:51` — release build 中不平衡缩进静默产生损坏输出。

### M19. NAR 路径穿越测试是空桩
`store/src/nar.rs` — 测试函数构建了恶意 header 但没有实际提取验证。

### M20. typeck 核心模块零单元测试
`infer.rs`、`unify.rs`、`traits.rs`、`pattern_analysis.rs` 仅通过集成测试覆盖，修改风险高。

### M21. eval 单元测试极少
`eval.rs` 仅有 1 个 `#[test]` 函数，核心求值逻辑完全靠集成测试。

### M22. parser shebang 测试 `#[ignore]` — 位置错误
`tests/parser.rs:1830` — shebang 剥离由 CLI 处理，测试应移到 CLI 层面或 parser 实现该功能。

---

## Low (技术债务)

### L1. tree-sitter-neve 不在 workspace 中
`tree-sitter-neve/bindings/rust/` 不在 `members` 列表中，不会被构建/测试/lint。

### L2. tree-sitter-neve build.rs 引用不存在的 parser.c
`tree-sitter-neve/bindings/rust/build.rs:9` — 需要先运行 `tree-sitter generate`。

### L3. `panic = "abort"` 可能破坏依赖的 catch_unwind
`Cargo.toml:28` — release profile 设 `panic = "abort"`，某些依赖(如 tokio)内部用 `catch_unwind`。

### L4. `u64 -> usize` 截断风险(32-bit)
`store/src/nar.rs:384` — 32 位平台 >4GB NAR 文件会截断。

### L5. Rc 指针强制转换为 usize 用于哈希
`eval/src/value.rs:1396-1484`, `eval/src/env.rs:119` — 依赖地址稳定性，脆弱但当前安全。

### L6. CI: 无覆盖率测量、无 benchmark、无 fuzz testing
没有 `cargo-tarpaulin`/`cargo-llvm-cov`，没有 criterion benchmark，没有 `libfuzzer-sys`。

### L7. CI: Windows 跳过大部分集成测试
builder、config、fetch、frontend、lsp、store、fmt 集成测试在 Windows 上不运行。

### L8. CI: `cargo audit` 用 `continue-on-error: true`
依赖漏洞不阻塞 CI 通过。

### L9. CI: 无 `cargo test --doc`、无 MSRV 检查
文档中的代码示例可能已经过时；声明的 MSRV 1.85 未被验证。

### L10. `FnDef.effect` 字段仍保留在 AST 中但关键字已移除
`neve-syntax/src/ast.rs:67` — effect 关键字已成为 legacy，但 AST 还带着这个字段。

### L11. neve-config ServiceUnit 用 String 而非 enum
`config/src/generate.rs:39` — `service_type: String` 允许任意字符串，应为 `ServiceType` enum。

### L12. neve-std I/O 有 14 对重复函数
`readFile`/`readFilePath`、`writeFile`/`writeFilePath` 等 — 两套 API 做同样的事。

### L13. neve-cli 有孤立的 `neve-lexer` 依赖
`neve-cli/Cargo.toml:15` — CLI 不直接使用 neve-lexer，但声明了依赖(edition 2024 bin target 是 warning)。

### L14. REPL 历史保存错误静默丢弃
`repl.rs:310, 106` — `let _ = save_repl_history(...)` 丢弃所有错误。

### L15. LSP 缺失 did_change_configuration/did_change_watched_files
`backend.rs` — 高级编辑器集成可能需要这些。

### L16. LSP code_action 只生成无操作的标签
`backend.rs:1070-1097` — 快速修复空有其表，没有任何实际编辑。

### L17. `neve test` 和 `neve run` 缺少 doc comment
`test.rs` (0 docs), `run.rs` (0 docs) — `neve test --help` 和 `neve run --help` 无帮助文本。

### L18. `--version` flag 和 `version` 子命令输出不一致
`main.rs:125,377-381` — `neve --version` 和 `neve version` 输出不同格式。

---

## 影响 E2E 的问题

以下问题有对应的 E2E 测试缺失或薄弱：

| 问题 | E2E 覆盖状态 | 建议 |
|------|-------------|------|
| H3 (trait bound 未强制) | 无测试 | 添加 `test_end_to_end_trait_bound_violation` |
| H4 (types_match 忽略参数) | 无测试 | 添加 `test_end_to_end_generic_impl_disambiguation` |
| H5 (enum 泛型参数丢弃) | 无测试 | 添加 `test_end_to_end_generic_enum_type_safety` |
| H8 (无递归深度限制) | 无测试 | 添加 `test_end_to_end_recursion_depth_limit` |
| H9 (occurs check 绕过) | 无测试 | 添加 `test_end_to_end_infinite_type_detection` |
| C6 (formatter 丢弃注释) | 假阳性 | 重写 `test_comment_preserved` |

---

## 修复优先级路线图

### 第 1 轮: 致命问题 — 立即修复
1. [ ] C1: 修复 reqwest TLS 特性覆盖 (2 行改动)
2. [ ] C6: Formatter 注释保留 (需要 AST 支持或 CST 层)
3. [x] C2/C3/C7: API 密封/可见性/命名冲突 (批量 `pub` → `pub(crate)` + `#[non_exhaustive]`) ✅ Fixed
4. [ ] C4/C5: README 和示例语法更新

### 第 2 轮: 高风险问题 — 下个迭代
5. [ ] H1/H2: 清理未使用的依赖 (edition 2024 合规)
6. [ ] H3/H4/H5/H9: 类型系统正确性修复 (trait bound, enum generics, occurs check)
7. [ ] H6: Mutex 中毒 (替换为 `parking_lot::Mutex` 或 `unwrap_or_else`)
8. [ ] H8: 递归深度限制 (在 eval 中添加计数器)
9. [ ] H12: UTF-8 路径 panic (`to_string_lossy()`)
10. [ ] H13: REPL Ctrl+C 保护 (添加信号处理器)

### 第 3 轮: 中等问题 — 持续改进
11. [ ] M1-M5: 依赖管理清理
12. [ ] M6-M13: 类型系统和 eval 改进
13. [ ] M14-M16: CLI/REPL/LSP 健壮性
14. [ ] M19-M22: 测试覆盖增强

### 第 4 轮: 低优先级 — 技术债务
15. [ ] L1-L3: tree-sitter 和构建配置
16. [ ] L6-L9: CI/CD 增强
17. [ ] L10-L18: API 清理和文档

---

## 分数卡

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计 | **B+** | 管道架构清晰，依赖图是无环 DAG。扣分: 依赖管理不干净 |
| 类型安全 | **B** | H-M + Trait 基础扎实。扣分: trait bound 未强制，enum generic 不完整 |
| 运行时安全 | **B-** | 无 unsafe 滥用，TCO 实现正确。扣分: 无递归限制，mutex 中毒风险 |
| 错误处理 | **C+** | 生产代码 `?` 传播一致。扣分: 429 个 unwrap, CLI 错误消息质量 |
| 测试覆盖 | **B** | 541 E2E 覆盖广。扣分: 核心模块缺单元测试，无 benchmark/fuzz |
| API 设计 | **C+** | 管道 facade 模式干净。扣分: AST 未密封，大量 pub 泄露 |
| 文档质量 | **C** | 结构完整。扣分: README 语法漂移，示例全部过时 |
| 开发体验 | **B-** | LSP 20 methods, REPL 实用。扣分: 错误消息不精确，comment 被丢弃 |
| **综合** | **B-** | 语言核心健壮，工程化不足。建议重点投入类型系统正确性和 API 稳定性 |

---

## AST/HIR/IR Review (2026-09-25)

按项目 v4.0/v5.0 语法设定重新核对 lexer → parser → AST → HIR → typeck → eval 全链路，重点检查 AST 解析覆盖与 IR 完整性。发现并修复：

| # | 位置 | 问题 | 修复 | 验证 |
|---|------|------|------|------|
| 1 | `neve-lexer/src/lexer.rs` | `/` 一律按绝对路径字面量扫描，`6/2` 被切成 `PathLit` | 记录 `last_token_kind`，仅在上一 token 不能结束操作数时进入路径扫描 | `tests/lexer.rs::test_compact_division_is_not_an_absolute_path` |
| 2 | `neve-hir/src/resolve.rs` | 枚举变体与已有全局同名时静默覆盖；与枚举自身同名被误报为重复变体 | 按 `(enum DefId, span)` 记录变体，冲突时给出精确诊断 | `tests/typeck.rs`（self-name / duplicate 两例） |
| 3 | `neve-eval/src/eval.rs` | 零参数项求值语义与 typeck 不一致（显式 `fn f() = …` 被当作可调用对象） | 统一为值绑定语义；移除临时引入的 `FnDef.is_binding` 字段 | `cargo test --workspace`、typeck 一致性 |
| 4 | `neve-eval/src/eval.rs` | `io.defer` 只在程序结束时统一执行，失败路径不执行 | 延迟动作归属调用帧，帧退出（成功或失败）即执行，并恢复外层作用域 | `tests/end_to_end.rs` 三个 defer 测试 |
| 5 | `neve-eval/src/value.rs` + `eval.rs` | thunk 失败后被缓存为 `Evaluated(Err(..))`，再次 `force` 得到 `Ok(Err(..))` 且状态与错误不一致 | 新增 `ThunkState::Failed(EvalError)`，失败结果原样缓存 | `eval::tests::test_force_caches_thunk_failure` |
| 6 | `neve-eval/src/eval.rs` | `ExprKind::MethodCall` 不在 `eval_with_tco` 分支中，方法调用尾递归退化为普通递归 | 方法调用解析拆分为 `resolve_method_call`，尾位置返回 `TcoResult::TailCall` | `tests/eval.rs::test_eval_method_call_tail_recursion_is_optimized`（11_000 层） |
| 7 | `neve-eval/src/eval.rs` | `MAX_RECURSION_DEPTH = 10_000` 实际不可达：单帧数 KB，约 1_000 层即原生栈溢出并 `abort`（debug/release 均复现） | `apply` 测量当前线程可用栈（`pthread_getattr_np`，512 KiB 保留，探测失败回退 512 KiB），预算耗尽即返回可诊断错误 | CLI 复现转为 `E0303`；`eval::tests::test_non_tail_recursion_reports_stack_budget_instead_of_aborting` |
| 8 | `neve-lsp/src/semantic_tokens.rs` | 列号按字符计数、长度按字节计算，astral 字符使同行后续 token 偏移 | `offset_to_line_col` / `utf16_length` 统一按 UTF-16 码元计算 | `tests/lsp.rs::test_semantic_tokens_positions_use_utf16_code_units` |
| 9 | `tests/end_to_end.rs` | `test_defer_execution_order` 为恒真断言（`assert!(ok || err)`） | 删除并替换为真实行为测试 | 同上 |

未修复（记录为后续边界）：

| 位置 | 现象 | 影响 |
|------|------|------|
| `neve-lexer` | 非 ASCII 标识符被拒绝（`E0001 unexpected character`） | 语言词汇表限定为 ASCII 标识符；如需 Unicode 标识符需单独立项 |
| `neve-eval` | 表达式深度（非 `apply` 路径）不受栈预算约束 | 极深嵌套表达式仍可能触及原生栈；当前由解析器先行失败覆盖 |
| `neve-typeck` / `neve-eval` | 零参数 `fn f() = …` 是值绑定，`f()` 不是合法调用 | 与 Rust 直觉不同，属既有语义；已在 changelog 与技能文档中明确 |

**安全复审（2026-09-25，security-reviewer，只读）**：本批次新增的 3 处 `unsafe`（`neve-common::kill_process` 的 `libc::kill`、`neve-eval` 的栈探测与信号处理、tree-sitter 的 `unsafe extern "C"`）逐条核对无越界读写、无 UB，平台回退为 fail-safe（预算偏保守，不会放过溢出）。无 critical/high。cargo audit / cargo deny / trivy / gitleaks 均通过（`validate.sh` Gate 1/5）。两条 low 属既有代码，**不由本批次引入**，留待后续：

| 位置 | 现象 | 建议 |
|------|------|------|
| `crates/neve-eval/src/builtin.rs` `json_to_value`（`builtin.rs:1637-1730`，`fromJSON` 入口） | 递归解析外部 JSON 无深度上限；不经过 `Evaluator::apply`，因此新增的栈预算守卫覆盖不到，深层嵌套 `[` 可在解析期耗尽原生栈 | 增加显式深度上限（例如 128）并返回可诊断错误；`value_to_json` 同样处理；加测试固定上限 |
| `crates/neve-common/src/lib.rs` `kill_process`（`lib.rs:30-40`） | `pid as i32` 隐式截断改变 `kill` 语义（Linux `pid_max ≤ 2^22`，实际不可达）；超时 kill 与 `child.wait_with_output()` 回收之间存在 PID 复用窗口 | `libc::pid_t::try_from` + 失败跳过；或由持有 `Child` 的线程终止 / Linux 上用 `pidfd_open` |

其他 info 级建议（不阻塞）：`install_signal_handler` 忽略 `libc::sigaction` 返回值；`DepthGuard` 的裸指针可改用 `&Cell<u32>` 持有以去掉 `unsafe`；注册 `io.onSignal` 会取代 TERM/HUP 的默认终止语义，需在文档中说明。

**独立审查（2026-09-25，agent://reviewer，只读）**：`CHANGES REQUESTED`，两条 Major 均属本次新增的按帧 `io.defer` 语义，已修复并补齐失败前/通过后的回归证据：

| # | 位置 | 问题 | 修复 | 证据 |
|---|------|------|------|------|
| 1 | `eval.rs` `eval_module` / `force_thunk` | 模块帧与 thunk 帧注册的 defer 永不执行（相对旧的“程序结束统一执行”是静默回归） | 模块第二遍求值包在 take/settle/restore 中；thunk 子求值器在丢弃前 settle | `test_defer_runs_at_module_scope`、`test_defer_runs_inside_thunk_frame`（禁用修复即失败） |
| 2 | `eval.rs` TCO 循环 | 尾位置的 defer 在被调用者执行前、以 FIFO 顺序运行，与同构非尾调用相反 | 帧 defer 挂起到 `pending_frame_defers`，链结束时由内层向外层 flush | `test_defer_order_matches_tail_and_non_tail_calls`、`test_defer_runs_when_tail_called_frame_fails`（禁用修复即失败） |
| 3 | `lexer.rs` `can_start_operand` | 漏掉后缀 `?`，`total?/2` 被词法成 `PathLit("/2")` | 操作数结束集合加入 `Question` | `tests/lexer.rs::test_slash_after_try_operator_is_division` |
| 4 | `eval.rs` `run_defers` | 单个 defer 失败会跳过同帧其余 defer，函数注释却称“全部执行” | 全部尝试，报告首个错误 | 与 #2 的测试共用路径 |
| 5 | `tests/end_to_end.rs` | `test_end_to_end_bytes_len` 恒真断言掩盖了源码缺少 `use std.io = io`（该用例从未真正通过类型检查）；retry 用例同样恒真 | bytes 用例改为 TempDir 真实读取并断言 `bytes.len(data) > 0`；retry 用例断言类型检查干净且 HIR 求值成功 | 两用例现均为真实断言并通过 |
| 6 | 文档/配置 | spec 的“标识符为 ASCII”与实现（ASCII 起始 + Unicode 续字符）矛盾；技能表引用不存在的 `neve-lexer/src/span.rs`、`neve-syntax/src/item.rs`、`neve-parser/src/{expr,pattern}.rs`；`.editorconfig` 对生成的 `parser.c` 用 4 空格；`semantic_tokens.rs` 存留死变量与失实注释 | 按实现改写 spec 与技能；表内路径改为实际文件并纳入 `verify-skills.sh` 校验；`parser.c` 单独声明 2 空格；删除死变量并修正注释 | `verify-skills.sh` 17 → 25 项检查全通过 |

**新增后续边界（非本批次引入，未修复）**：

| 位置 | 现象 | 影响 / 建议 |
|------|------|-----------|
| `neve-typeck/src/traits.rs` `resolve_method` trait 回退分支 | 在 `trait_impls`（HashMap）上取首个匹配的 impl，迭代顺序随进程随机种子变化 | 多个候选 impl 且接收者仍是类型变量时，方法选择不确定且被写进运行时分派表；需确定性排序 + 多匹配歧义诊断。该循环早于本批次存在（`git diff --cached` 中为上下文行），本批次的替换匹配扩大了可匹配面 |
| `crates/neve-eval/src/diagnostics.rs` | 递归上限与栈预算以 `TypeError` → E0303 “runtime type error” + “加个类型标注”帮助呈现，且 >80 字符的消息被截断 | 建议为递归/栈耗尽增加独立 `EvalError` 变体与 `ErrorCode`（会新增第 56 个错误码，需同步 codes 表与文档）；本批次先把消息缩短到不被截断 |
| `tree-sitter-neve/grammar.js` | 仍是 v3.x 形态（`if … then … else`、`lazy`、强制 `{ … }` 枚举、无后缀 `?`），而 `src/parser.c` 已随本批次重新生成 | 编辑器语法与编译器行为分歧；需按 v4.0 规范补规则并重新 generate |

---

## 最新验证状态 (v5.0.0)

**Current status (2026-09-24):** The v5.0.0 AST boundary cutover is implemented and verified: public AST enums are non-exhaustive, unsupported AST-to-HIR forms retain explicit diagnostics, and top-level refutable `let` patterns without bindings are rejected instead of being silently dropped. Remaining follow-up boundaries are comment trivia preservation, tree-sitter native generation, record-variant semantics, duplicate variant identity, and tooling mappings.

The release fixes documented above remain historical evidence. New changes MUST be validated against the current source and targeted behavior, not this ledger alone.

**Follow-up review (2026-09-23):** `Cargo.lock` was refreshed and `git2` was upgraded to `0.21.0` for current RustSec advisories. The canonical pipeline now has 2,690 passing workspace tests; the custom enum coalesce evaluator regression is fixed and covered. Remaining AST/HIR review findings are short-circuit evaluation, declaration-order-independent effect inference, effect traversal in guards/comprehensions, `std.<module>.<item>` lowering, lazy parameter preservation, and evaluator environment restoration after errors.
