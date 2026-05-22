<div align="center">

<img src="../../assets/logo.svg" width="120" alt="Neve logo">

<h1>Changelog</h1>

<p><em>更新日志</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>

Based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
基于 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)。

---

> *What changed, when, and why.*  
> 更新日志：记录改变、时间和原因。

## [3.15.0] - 2026-05-22

### Added
- **LSP overhaul**: 19 LSP methods implemented -- hover, completion (type-aware), signatureHelp, definition, references, documentHighlight, rename, formatting, documentSymbol, semanticTokens (AST-based), inlayHint, foldingRange, codeAction, completionItem/resolve.
- **Type-aware completion**: Methods filtered by receiver type (List:32, String:16, Option:11, Result:9, Record:3). Uses DefId resolution via ModuleSemantics.global_names.
- **Signature help**: 80 builtin function signatures + user-defined functions via AST inspection.
- **Completion documentation**: 77 functions with docs via completionItem/resolve.
- **AST-based semantic tokens**: 8 node kinds (fn, struct/enum/trait/impl items, fields, variants, params, imports).
- **Helix integration**: 6 query files (22 highlight scopes), `neve setup helix` one-shot install.
- **VS Code extension**: TextMate grammar, LSP client, `neve setup vscode`.
- **CLI**: `neve lsp --check` (10-point health check), `neve lsp --version`.
- **Scripts**: build-grammar.sh (cross-platform), dev-setup.sh (one-shot).
- **Docs**: docs/reference/lsp.md.

### Changed
- **Highlights.scm**: 6 generic scopes -> 22 fine-grained Helix-compatible scopes.
- **Semantic tokens**: Lexer-only -> AST-based with lexer fallback.
- **languages.toml**: Removed broken formatter command; rely on LSP formatting.
- **Completion ordering**: Local symbols first, then stdlib, types, keywords.
- **AGENTS.md**: Updated LSP status (~98%) and priority items.

### Fixed
- **DefId resolution**: type_to_name now resolves Named types through ModuleSemantics.global_names.
- **setup_helix.rs**: Uses include_str!() for all 6 query files (was hardcoded stale strings).

### Tests
- **LSP**: 164 tests (151 integration + 13 unit).
- **E2E**: 8 new end-to-end tests (lsp_e2e.rs).

## [3.8.0] - 2026-05-12

### Added / 新增
- **Phase 4 complete**: Shell Capability Replacement — Stream<T> 14 APIs, TTY 4 APIs, Job control 2 APIs.
- **Stream<T>**: 14/14 APIs implemented (Phase A-C): streamList/streamLines/streamCommand/streamBytes, streamMap/streamFilter/streamTake/streamDrop, streamCollect/streamPipe/streamWrite/streamForEach/streamFold, streamWithTimeout.
- **Task APIs**: 7 (spawn/poll/cancel/await/awaitTasks/awaitAny/awaitTaskWithTimeout).
- **TTY APIs**: 4 (isTTY/terminalSize/setRawMode/resetTerminal).
- **Job control**: 2 (jobs/waitAnyJob).
- **Formal verification v4**: 19 Lean modules, EffectEval v4.3 (34 rules, +5 stream Phase C), BinOp 12/12 proved.
- **Bytes type formalization**: Ty.Bytes, Value.bytes, canonical forms, EffectEval rules, refinement bridge.
- **BigStep v2**: matchOn_fallthrough rule, div_zero/mod_zero rules (27 total rules).
- **SafetyLemmas**: 5 verified pattern matching lemmas (wildcard, lit_int, bool_full, unit, bool_first_arm).
- **Effect boundary document**: docs/project/effect-boundary.md v1.0 (G4 decision gate closed).
- **CI bug hunter**: .github/workflows/bug-hunter.yml (nightly + push/PR + manual trigger).
- **Formatter idempotency**: 37/37 tests pass (crates/neve-fmt/tests/idempotency.rs).
- **Pipeline**: |> syntax (AST=HIR=typeck parity).
- **Example scripts**: examples/test-runner.neve, examples/ci-bootstrap.neve.
- **CHANGELOG.md**: Added at repository root.
- **Phase 5 items**: Ecosystem design doc, stability tiers (Tier 1/2/3), flake/lock/store, registry CLI (17 commands).

### Changed / 变更
- **EnvMatches**: Refactored to predicate-parameterized EnvMatches(P) in Values.lean.
- **Type safety v18**: env_preservation lemma extracted; app/pipe non-lam cases documented.
- **EffectEval**: v4.3 (34 rules): +5 stream Phase C, +cancel/awaitAny, +retry/ensure, +awaitTasks/timeout.
- **Retry/Ensure**: EffectEval rules added (retry_success, retry_failure, ensure_success, ensure_timeout).
- **kill_process**: Moved to neve-common as single source of truth (M-2 unified kill mechanism).
- **G5 (Bash Replacement)**: Decision gate closed ✅.
- **Clippy**: 0 warnings across workspace.

### Tests / 测试
- **E2E**: 400 (+180 from 220): Stream Phase C (7), Job control (2), TTY (2), plus prior Stream/Task/pipe/redirect expansion; Phase 5 ecosystem (77 from 323→400).
- **Formatter**: 37 idempotency tests (37/37 pass).
- **Ecosystem**: builder(6), store(40), fetch all passing.

## [3.7.0] - 2026-05-09

### Added / 新增
- **Formal verification**: 14 Lean modules, lake build clean. Type safety theorem, EffectEval v3 (15 rules).
- **Security proofs**: Verify/Path (M-1), Verify/Environ (M-4), Verify/Limits (H-1, H-2) — machine-checked.
- **Differential testing**: 300+ random pure expression tests (ALL MATCH), effects property tests, CI integration.
- **Bug hunter**: scripts/bug_hunt.py — 11/11 security boundary attacks, 0 real bugs found.
- **Spec v2.2**: docs/reference/spec.md Part II — formal semantics documented.

### Changed / 变更
- **Security fixes**: MAX_STDIN_BYTES (10MB), MAX_OUTPUT_BYTES (50MB) checks in all 5 blocking execution paths.
- **Path safety**: resolve_redirect_path rejects .. components (M-1).
- **Env safety**: configured_process_command strips LD_PRELOAD/DYLD_* (M-4).
- **Workspace version**: 3.4.2 → 3.7.0 (bumped in final commit).

### Fixed / 修复
- **L-3**: 6 missing builtins implemented (walk, chmod, chown, symlink, readlink, tempDir).

## [3.6.0] - 2026-05-09

### Added / 新增
- **Structured io.args()**: Returns `(List<String>, Record)` tuple for pattern destructuring.
  Supports `-flag` → Bool, `-j8` → Int, `-f out` → String, `-10` → positional, `--` separator.
- **File operations**: `io.tempDir(fn)` auto-cleanup, `io.walk(dir)` recursive traversal,
  `io.chmod`, `io.chown` (Unix), `io.symlink`, `io.readlink`.
- **Environment**: `io.setEnv`, `io.unsetEnv` for process-level env management.
- **Non-blocking timeout**: `io.spawnWithTimeout(task, ms)` — auto-cancel on timeout.
- **TTY control**: `io.isTTY(fd)`, `io.terminalSize()` (Unix).
- **Pipeline spawn**: `io.spawn` now supports `Task[Pipeline]` in addition to `Task[Command]`.
- **Command pipe chaining**: `cmd1 |> cmd2 |> cmd3` via Pipeline append on `|>`.

### Changed / 变更
- `io.args()` return type changed from `List<String>` to `(List<String>, Record)`.
- Feature matrix: shebang/argv upgraded to ✅; glob, record match, match exhaustiveness updated.
- Phase 2 script polishing: 10/10 items complete per the Better Bash roadmap.

### Fixed / 修复
- `io.args`: negative numbers (`-10`) no longer parsed as flags.
- `io.args`: compact form (`-j8`) now supported.
- `io.args`: single dash (`-`) correctly treated as positional.

---

## [3.5.0] - 2026-05-08

### Added / 新增
- **Streaming timeout**: `io.execCommandStreamingWithTimeout` / `io.execPipelineStreamingWithTimeout` — streaming execution with total deadline + process kill on timeout.
- **Streaming safety limits**: max lines (100k), max stdin (10MB), max intermediate buffer (50MB) enforced in evaluator; path canonicalization for `readFileLines`.
- **Real signal handling**: `io.onSignal("INT"|"TERM"|"HUP"|"USR1"|"USR2", fn)` — OS-level signal handlers with atomic flags, dispatched at evaluator safe points.
- **Non-blocking Task**: `io.spawn(task)` / `io.poll(id)` / `io.cancel(id)` — background Command/Pipeline execution with global spawn registry.
- **Command pipe syntax**: `cmd1 |> cmd2` — creates Pipeline from Commands; `cmd1 |> cmd2 |> cmd3` chains through Pipeline.
- **std.bytes module**: `bytes.len` / `bytes.isEmpty` / `bytes.concat` / `bytes.fromString` / `bytes.toString` / `bytes.toList` / `bytes.fromList`.
- **eventMap/eventFilter**: Event chaining now stores function/predicate for lazy transformation at poll time.
- **Variable tab completion**: REPL now auto-completes user variables, functions, builtins, keywords, and type names.

### Changed / 变更
- **"undefined" suggestions**: Unknown names now produce "did you mean X?" via Levenshtein distance (e.g., `pront` → `did you mean 'print' (distance 2)?`).
- **LSP cross-module types**: Hover now shows readable type names (`List<Int>`) for stdlib and imported types via `global_names` in `ModuleSemantics`.
- **Match exhaustiveness**: Extended to `List<T>` (empty/non-empty coverage) and `Tuple` (per-position coverage). Previously marked `NotAnalyzed`.
- **Formatter idempotence**: Fixed trailing semicolons on struct/enum/trait/impl and match arm arrow (`=>` → `->`). 15 idempotence tests added.
- **Signal handler**: `libc::signal` → `libc::sigaction` for portable semantics; `Relaxed` → `Release/Acquire` ordering for signal flags.
- **Architecture**: `check.rs` (4848 lines) split into `check/mod.rs` (3401) + `check/builtin_type.rs` (1494).

### Fixed / 修复
- Pipeline timeout **deadlock** in error paths: removed blocking `result_rx.recv()`, added 500ms safety timeout.
- `check_signals` **performance**: eliminated `HashMap::clone()` on every `apply()` call.
- `kill_process_by_pid` now uses `libc::kill()` directly instead of spawning `kill` subprocess.
- `io.onSignal` callback arity validation; OS handler installed only once per signal name.
- Missing `io.awaitTaskWithTimeout` type signature added.
- **Pre-existing test failures** resolved: `binding_pattern_runtime_parity`, `repl_hir_runtime_preserves_project_module_namespace_imports`.
- Parser golden tests: 29 new tests added, 3 of 6 known gaps closed (crate import, multi-line comment, effect on impl method).

### Security / 安全
- Phase 3 security audit: streaming timeout, size limits, path validation, signal safety, effect classification fixes.

---

## [3.2.0] - 2026-05-04

### Added / 新增
- **Effect system**: Added `effect` keyword for explicit side-effect annotation. Pure functions are checked by default; `--allow-effects` bypasses the check.
- **Scripting builtins**: `io.execCommandLines` (line-by-line output), `io.awaitTaskWithTimeout` (timeout + process kill for Command/Pipeline), `io.args()` (script arguments), `io.env()` (environment variables), `io.sleep()`, `io.which()`.
- **Shebang support**: `.neve` files starting with `#!/usr/bin/env neve` can be executed directly. `neve run file.neve arg1 arg2` passes arguments via `io.args()`.
- **Pipeline timeout**: `io.awaitTaskWithTimeout` now supports Pipeline tasks with per-stage process kill.
- **Windows process kill**: Timeout-based process termination uses `taskkill` on Windows, `kill -9` on Unix.
- **Example scripts**: `manifest.neve` (file listing + SHA-256), `build-check.neve` (CI checks), `http-check.neve` (HTTP health), `backup.neve` (file backup with timestamp).

### Changed / 变更
- **Effect checking is default-on**: `neve check` now rejects effectful calls by default. Use `--allow-effects` to restore permissive behavior. The old `--pure` flag is removed.
- **Spec v2.1**: Updated language specification with Effect System and Scripting chapters. Keywords: 20 → 21 (added `effect`).

### Improved / 改进
- **Phase B exit criteria validated**: Build reproducibility, lockfile determinism, and GC safety all verified with real package builds.
- **Formatter**: Fixed `effect` keyword being dropped during formatting. Verified idempotent.
- **Parser golden tests**: 189 tests including new syntax forms (effect annotation, list comprehension, safe field, try expression).
- **Cross-platform**: Process kill works on both Unix and Windows.

### Fixed / 修复
- **Formatter**: `effect` keyword was silently dropped during formatting (now preserved).
- **Unreachable pattern diagnostics**: Added missing `ErrorCode::UnreachablePattern` to unreachable pattern warnings.
- **Pipeline timeout**: Previously unsupported; now fully implemented.

## [3.1.0] - 2026-04-18

### Added / 新增
- **Typed `std.math` canonical surface**: Added explicit public support for `math.toInt`, `math.toFloat`, `math.isNan`, `math.isInf`, `math.floor`, `math.ceil`, `math.round`, `math.sqrt`, `math.log`, `math.log10`, `math.exp`, `math.sin`, `math.cos`, and `math.tan`, with aligned type checking, runtime behavior, REPL `:type`, LSP hover/completion, API docs, and regression coverage. / **类型化 `std.math` 规范表面**: 新增 `math.toInt`、`math.toFloat`、`math.isNan`、`math.isInf`、`math.floor`、`math.ceil`、`math.round`、`math.sqrt`、`math.log`、`math.log10`、`math.exp`、`math.sin`、`math.cos` 与 `math.tan` 的显式公开支持，并同步打通类型检查、运行时行为、REPL `:type`、LSP hover/completion、API 文档与回归覆盖。

### Improved / 改进
- **Stdlib tooling truthfulness**: Reorganized LSP stdlib completion metadata into surface-specific modules with regression tests, and aligned completion/hover/type-query output with the real verified `io`, `list`, `string`, `path`, `fetch`, `Map`, `Set`, `option`, `result`, and `math` surfaces instead of stale placeholders or runtime-only entries. / **标准库工具链真实性**: 将 LSP 标准库补全元数据重组为按表面拆分的模块并补齐回归测试，同时让 completion / hover / 类型查询重新对齐真实已验证的 `io`、`list`、`string`、`path`、`fetch`、`Map`、`Set`、`option`、`result` 与 `math` 表面，不再暴露陈旧占位项或仅运行时存在的入口。
- **Frontend-owned REPL display attribution**: `FrontendSession` now owns canonical current-input display attribution for REPL flows, including `<repl>` / `<repl:type>` naming, file-backed input display names, and checked-source display projection, reducing remaining CLI-local duplication across ordinary inputs, `:type`, and `:load`. / **frontend 持有的 REPL 诊断归属**: `FrontendSession` 现在接管 REPL 当前输入的规范展示归属，包括 `<repl>` / `<repl:type>` 命名、文件输入展示名以及 checked-source 的 display projection，进一步减少普通输入、`:type` 与 `:load` 路径上的 CLI 本地重复逻辑。

## [3.0.0] - 2026-04-15

### Added / 新增
- **Object-carried process/runtime mainline**: Added the first verified public `Pipeline`, `Redirect`, `Task[ProcessResult]`, and `Bytes` bridges on the canonical pipeline, including `io.pipeline`, `io.pipelineWithRedirects`, `io.redirectStdoutPath` / `io.redirectStderrPath` / `io.redirectStdinPath`, `io.taskCommand`, `io.taskPipeline`, `io.awaitTask`, `io.awaitTasks`, and binary-safe file bridges such as `io.readFileBytesPath`, `io.writeFileBytesPath`, and `io.appendFileBytesPath`. / **对象承载的进程/运行时主线**: 在规范主线上新增首批已验证的公开 `Pipeline`、`Redirect`、`Task[ProcessResult]` 与 `Bytes` 桥接，包括 `io.pipeline`、`io.pipelineWithRedirects`、`io.redirectStdoutPath` / `io.redirectStderrPath` / `io.redirectStdinPath`、`io.taskCommand`、`io.taskPipeline`、`io.awaitTask`、`io.awaitTasks`，以及 `io.readFileBytesPath`、`io.writeFileBytesPath`、`io.appendFileBytesPath` 等二进制安全文件桥。
- **Expanded typed path bridges**: Added a wider typed `Path` mainline across `std.path` and `std.io`, including pure adapters like `path.filenamePath` / `path.extensionPath` and host-boundary bridges such as `io.homeDirPath`, `io.createDirAllPath`, `io.removeDirAllPath`, `io.hashFilePath`, `io.writeFilePath`, `io.appendFilePath`, and `io.readDirEntryPaths`. / **扩展 typed `Path` 桥接**: 在 `std.path` 和 `std.io` 中补齐更宽的 typed `Path` 主线，包括 `path.filenamePath` / `path.extensionPath` 这类纯内存 adapter，以及 `io.homeDirPath`、`io.createDirAllPath`、`io.removeDirAllPath`、`io.hashFilePath`、`io.writeFilePath`、`io.appendFilePath`、`io.readDirEntryPaths` 等 host-boundary bridges。
- **Tooling/documentation parity hardening**: Added broad consumer parity for the new mainline across REPL `:type`, LSP hover/completion, API docs, feature matrix, and the semantic convergence log, so canonical `std.io` / `std.path` / `std.list` / `std.fetch` / `Map` / `Set` / `string` / `option` / `result` / `math` surfaces now describe verified reality instead of stale placeholders. / **工具链与文档一致性加固**: 为新的主线补齐了 REPL `:type`、LSP hover/completion、API 文档、feature matrix 与 semantic convergence log 的广泛一致性覆盖，让 canonical `std.io` / `std.path` / `std.list` / `std.fetch` / `Map` / `Set` / `string` / `option` / `result` / `math` 表面与已验证现实重新对齐。

### Changed / 变更
- **Canonical execution surface is now object-first**: Public process execution now converges on `io.command(...)` / `io.commandWith(...)` / `io.commandWithRedirects(...)` plus `io.execCommand(...)`, and `io.pipeline(...)` / `io.pipelineWithRedirects(...)` plus `io.execPipeline(...)`; redirect execution now rides on first-class `Command` / `Pipeline` objects instead of separate wrapper paths. / **规范执行面已转为对象优先**: 公开进程执行现在收敛到 `io.command(...)` / `io.commandWith(...)` / `io.commandWithRedirects(...)` 配合 `io.execCommand(...)`，以及 `io.pipeline(...)` / `io.pipelineWithRedirects(...)` 配合 `io.execPipeline(...)`；redirect 执行也改为附着在一等 `Command` / `Pipeline` 对象上，而不是独立 wrapper 路径。
- **Pipeline/redirect boundaries became explicit**: Pipeline construction and redirect composition now reject invalid topology earlier, including empty pipelines, non-final `stdout` redirect, non-first `stdin`, duplicate stream redirects, and boundary/stage-local redirect conflicts. / **Pipeline/redirect 边界显式化**: pipeline 构造与 redirect 组合现在会更早拒绝无效拓扑，包括空 pipeline、非最终 stage 的 `stdout` redirect、非首 stage 的 `stdin`、重复 stream redirect，以及 boundary/stage-local redirect 冲突。

### Removed / 移除
- **Legacy compat execution wrappers**: Removed public compat wrappers `io.exec`, `io.execWith`, `io.execShell`, `io.execResult`, `io.execWithResult`, `io.execShellResult`, and the old boundary redirect execution wrappers. Shell behavior is still available, but now only through explicit `Command` construction such as `io.command("sh", ["-c", ...])` or `io.command("cmd", ["/C", ...])`. / **旧兼容执行 wrapper**: 移除了公开 compat wrapper：`io.exec`、`io.execWith`、`io.execShell`、`io.execResult`、`io.execWithResult`、`io.execShellResult`，以及旧的 boundary redirect 执行 wrapper。shell 语义仍可用，但现在只能通过显式 `Command` 构造表达，例如 `io.command("sh", ["-c", ...])` 或 `io.command("cmd", ["/C", ...])`。

### Improved / 改进
- **Full-pipeline release baseline**: The release was validated against `cargo build -p neve`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`, in addition to the narrower convergence slices recorded in `semantic-convergence-plan.md`. / **全流水线发布基线**: 本次发布除 convergence plan 中的窄切片验证外，还通过了 `cargo build -p neve`、`cargo test --workspace` 与 `cargo clippy --workspace --all-targets -- -D warnings` 的完整发布基线。

## [2.0.0] - 2026-04-15

### Added / 新增
- **Runtime object bridges for system automation**: Added first-class `Path`, `Command`, and `ProcessResult` runtime-object bridges in the verified mainline, including `std.path`/`std.io` entrypoints and aligned REPL/LSP/runtime type visibility. / **系统自动化运行时对象桥接**: 在已验证的主线中加入一等 `Path`、`Command` 与 `ProcessResult` 运行时对象桥接，并同步打通 `std.path`/`std.io` 入口以及 REPL/LSP/运行时类型可见性。
- **Canonical command/process surface starters**: Added `io.command`, `io.execCommand`, `io.processSuccess`, `io.processStdout`, `io.processCode`, and `io.processStderr` as the first explicit public bridges over the command/process runtime family. / **规范命令/进程公开起点**: 新增 `io.command`、`io.execCommand`、`io.processSuccess`、`io.processStdout`、`io.processCode` 和 `io.processStderr`，作为命令/进程运行时家族的首批显式公开桥接。
- **Full-pipeline convergence coverage**: Added focused regression coverage across `std`, `typeck`, `frontend`, `eval`, `end_to_end`, `lsp`, and REPL for the canonical runtime-object and tooling paths. / **全流水线收敛覆盖**: 为规范运行时对象和工具链主路径新增覆盖 `std`、`typeck`、`frontend`、`eval`、`end_to_end`、`lsp` 与 REPL 的聚焦回归测试。

### Improved / 改进
- **Canonical frontend/HIR convergence**: Mainline CLI/tooling workflows continue to converge on frontend-owned semantics, and legacy `io.exec` is now internally projected from the canonical `Command -> ProcessResult` execution path while preserving its public record contract. / **规范 frontend/HIR 收敛**: 主线 CLI/工具链工作流继续向 frontend 自有语义收拢，旧 `io.exec` 现在也已在内部改为从规范 `Command -> ProcessResult` 执行路径投影而来，同时保持原有 record 公开合同不变。
- **Type-system and diagnostics hardening**: Expanded builtin named runtime types, associated-type/pattern/optional-flow analysis, and readable type presentation across diagnostics, REPL `:type`, and LSP hover. / **类型系统与诊断加固**: 扩展内置命名运行时类型、关联类型/模式/optional-flow 分析，并提升诊断、REPL `:type` 与 LSP hover 的类型可读性。
- **Platform/config semantic truthfulness**: Build/config evaluation and project documentation were synchronized more tightly with the verified canonical path and explicit compatibility boundaries. / **平台/配置语义真实性**: build/config 求值路径与项目文档更紧密地同步到已验证的规范主路径和显式兼容边界。

### Fixed / 修复
- **Implicit semantic drift**: Reduced remaining hidden divergence between legacy command-local execution paths and the canonical runtime-object pipeline by routing `io.exec` through the shared command/process execution core. / **隐式语义漂移**: 通过让 `io.exec` 走共享的命令/进程执行核心，进一步减少旧命令局部执行路径与规范运行时对象流水线之间的隐藏分裂。

## [1.2.0] - 2026-04-14

### Added / 新增
- **Shared frontend driver/session**: Added multi-module `FrontendDriver` and compatibility `FrontendSession` so `check`, REPL, and LSP can consume one frontend-owned semantic artifact instead of rebuilding their own pipelines. / **共享前端驱动与会话**: 新增多模块 `FrontendDriver` 与兼容式 `FrontendSession`，让 `check`、REPL 和 LSP 开始共享同一份 frontend 语义产物，而不是各自重建语义流水线。
- **Frontend-owned integration coverage**: Added direct integration tests for the new driver/session layer, including module loading, import binding resolution, diagnostics attribution, and canonical higher-order `std.list` runtime parity. / **前端自有集成覆盖**: 为新的 driver/session 层补充直接集成测试，覆盖模块加载、导入绑定解析、诊断归属，以及规范 `std.list` 高阶运行时一致性。
- **File tail expression support**: Modules can now end with a trailing top-level expression, and the canonical HIR run path evaluates that final form consistently. / **文件尾表达式支持**: 模块现在可以以尾部顶层表达式结束，规范 HIR 运行路径会一致地执行这一最终 form。

### Improved / 改进
- **Canonical CLI convergence**: `neve check` and the canonical HIR path in `neve run` now use frontend driver results instead of command-local `ModuleLoader + TypeChecker` orchestration. / **规范 CLI 收敛**: `neve check` 与 `neve run` 的规范 HIR 主路径现在直接使用 frontend driver 结果，不再在命令层手工拼装 `ModuleLoader + TypeChecker`。
- **REPL/LSP semantic consistency**: REPL `:type` and LSP hover now read frontend-owned side tables and display names, reducing semantic drift across CLI, REPL, and editor tooling. / **REPL/LSP 语义一致性**: REPL `:type` 与 LSP hover 现在直接读取 frontend 维护的 side tables 和类型展示名，减少 CLI、REPL 与编辑器工具链之间的语义漂移。
- **Module infrastructure split**: `ModuleLoader` internals were decomposed into incremental cache, module path resolution, module graph bookkeeping, diagnostics storage, and module lowering helpers, preparing the path toward content-hash-based canonical loading. / **模块基础设施拆分**: `ModuleLoader` 内部已拆分为增量缓存、模块路径解析、模块图状态、诊断存储和模块 lowering 辅助层，为后续按稳定内容哈希收口的规范加载路径做准备。
- **Explicit AST compatibility boundary**: AST evaluation is now surfaced through explicit compat paths, and repository-internal callers use `neve_eval::compat` instead of treating AST evaluation as a peer default API. / **显式 AST 兼容边界**: AST 求值现在通过显式 compat 路径暴露，仓库内部调用方统一改用 `neve_eval::compat`，不再把 AST 当作默认对等 API。
- **HIR builtin convergence**: The HIR evaluator now owns the canonical runtime path for existing higher-order `std.list` entrypoints such as `list.map` and `list.filter`, reducing residual AST/HIR runtime divergence. / **HIR builtin 收敛**: HIR evaluator 现在接管已有高阶 `std.list` 入口（如 `list.map`、`list.filter`）的规范运行时路径，进一步减少 AST/HIR 运行时分裂。

### Fixed / 修复
- **Hidden backend fallback**: `neve run` / `neve eval` no longer silently switch to AST behavior; unsupported paths now fail clearly unless explicit compat mode is requested. / **隐藏后端回退**: `neve run` / `neve eval` 不再静默切回 AST；未覆盖路径现在会明确失败，除非用户显式请求 compat 模式。

## [1.1.1] - 2026-04-07

### Improved / 改进
- **Release truthfulness**: Rewrote the root README and install path so the public entry points, source install flow, and project status now match the real repository state. / **发布信息真实性**: 重写根 README 与安装路径，让公开入口、源码安装方式和项目状态描述重新与仓库真实状态保持一致。
- **Documentation status sync**: Corrected the feature matrix and language roadmap so end-to-end coverage is described as a real smoke baseline instead of stale placeholder text. / **文档状态同步**: 修正 feature matrix 与 language roadmap，对端到端覆盖的描述不再沿用过时的“占位实现”说法，而是准确标记为真实 smoke baseline。
- **REPL test hygiene**: Moved REPL-only helper logic fully under test scope to keep the runtime surface cleaner. / **REPL 测试卫生**: 将 REPL 专用辅助逻辑完整收回测试作用域，减少正式运行路径上的测试残留代码。

### Fixed / 修复
- **`neve doc` pager fallback**: Fixed terminal documentation viewing when `PAGER` points to commands like `cat`, by avoiding invalid pager flags and checking pager exit status before suppressing direct output fallback. / **`neve doc` 分页器回退**: 修复 `PAGER=cat` 等环境下的终端文档查看失败问题，不再向非 `less` 分页器传递无效参数，并会在分页器失败时正确回退到直接输出。
- **Release metadata consistency**: Updated CLI/release wording to match Neve's current positioning as a standalone language for system configuration and structured shell automation. / **发布元数据一致性**: 更新 CLI 与 release 文案，使其与 Neve 目前“面向系统配置与结构化 shell 自动化的独立语言”定位保持一致。

## [1.1.0] - 2026-04-07

### Added / 新增
- **Incremental HIR REPL**: REPL execution now keeps persistent HIR session state, including cross-input method dispatch, top-level redefinition, project-local module imports, relative module loading via `:load`, imported-module diagnostics, and safe root switching after `:clear`. / **增量 HIR REPL**: REPL 现在保留持久 HIR 会话状态，支持跨输入方法派发、顶层重定义、项目内模块导入、通过 `:load` 的相对模块加载、导入模块诊断，以及 `:clear` 后的安全根目录切换。
- **LSP semantic tooling**: Added semantic hover for references/expressions and scope-aware navigation for definition/reference/rename flows. / **LSP 语义工具**: 新增引用点与表达式级 hover，并让 definition/reference/rename 的导航开始按真实作用域解析。
- **System stdlib primitives**: Added structured `std.io` process execution, configurable execution, file writes/appends, and recursive directory lifecycle helpers. / **系统标准库原语**: 新增结构化 `std.io` 进程执行、可配置执行、文件写入/追加，以及递归目录生命周期辅助函数。

### Improved / 改进
- **Canonical HIR path coverage**: `neve eval`/`neve run` now prefer frontend/HIR across local imports and common `std` item/module/glob imports, reducing AST fallback on the main CLI paths. / **规范 HIR 主路径覆盖**: `neve eval`/`neve run` 现在在本地导入和常见 `std` item/module/glob 导入场景下优先走 frontend/HIR，减少主 CLI 路径上的 AST 回退。
- **Semantic convergence**: HIR lowering/runtime now preserve `lazy`, `?`, `??`, method calls, `or`/binding/list-rest patterns, block `let` patterns, and more associated-type use sites with better AST/HIR parity. / **语义收敛**: HIR lowering/runtime 现已更完整保留 `lazy`、`?`、`??`、方法调用、`or`/绑定/list-rest 模式、块级 `let` 模式，以及更多关联类型 use-site，AST/HIR 一致性更好。
- **Type system coverage**: Expanded typed stdlib coverage for `list`/`string`/`option`/`result`/`path`/`io`/`fetch`/`map`/`set`, trait impl checking, builtin `Option/Result` pattern analysis, and REPL/tooling type queries. / **类型系统覆盖**: 扩展了 `list`/`string`/`option`/`result`/`path`/`io`/`fetch`/`map`/`set` 的类型化覆盖，并强化了 trait impl 检查、内置 `Option/Result` 模式分析，以及 REPL/工具链类型查询。
- **Tooling readability**: Diagnostics and type displays now render imported named types readably across `check`/`run`/REPL, instead of leaking raw `Type#...` placeholders. / **工具链可读性**: `check`/`run`/REPL 的诊断和类型展示现在能把导入类型显示成人类可读名称，不再泄漏 `Type#...` 占位符。
- **Project truthfulness**: Added a feature matrix and a more explicit language roadmap so documented project status better matches the real compiler/runtime state. / **项目状态透明度**: 新增 feature matrix 和更明确的语言路线图，让文档中的项目状态更接近真实编译器/运行时现状。

### Fixed / 修复
- **AST recursion regression**: Restored self-recursive AST function evaluation. / **AST 递归回归**: 修复 AST 路径下自递归函数失效的问题。
- **Top-level type refinement**: Fixed type checker refinement for top-level bound values such as record field access after binding. / **顶层类型细化**: 修复顶层绑定值的类型细化问题，例如绑定后记录字段访问。
- **Placeholder test coverage**: Replaced placeholder end-to-end coverage with real runtime-parity tests. / **占位测试覆盖**: 将原先的占位端到端测试替换为真实运行时一致性测试。

## [1.0.1] - 2026-03-20

### Added / 新增
- **Store metadata registration**: Cache fetch and builder outputs now record path metadata into the store database. / **Store 元数据登记**: 缓存拉取与构建输出现在会把路径元数据登记进 store 数据库。
- **Cache roundtrip tests**: Added local and remote roundtrip coverage for `add_content` and `add_dir` path fetches. / **缓存回归测试**: 新增 `add_content` 与 `add_dir` 的本地/远程 roundtrip 拉取覆盖。

### Improved / 改进
- **Cache closure reliability**: Hardened signature checks, retry behavior, and closure fetch handling for binary cache downloads. / **缓存闭包可靠性**: 强化二进制缓存下载时的签名检查、重试行为与 closure 拉取流程。
- **Recursive fetch efficiency**: Reused store DB handles across recursive fetch operations and backfilled metadata for existing references. / **递归拉取效率**: 递归拉取流程复用 store DB 句柄，并为已有引用路径补全元数据。

### Fixed / 修复
- **Registration ordering bug**: Prevented early DB registration before final hash validation during fetch. / **登记顺序缺陷**: 修复拉取流程中在最终哈希校验前提前写入数据库的问题。
- **Hash compatibility bug**: Fetch verification now accepts store-native hash format to avoid false mismatch failures. / **哈希兼容性缺陷**: 拉取校验现在兼容 store 原生哈希格式，避免误报不匹配。

## [1.0.0] - 2026-01-31

### Added / 新增
- **Arbitrary-precision integers**: `Int` is now BigInt across lexer/parser/eval/typeck/stdlib. / `Int` 升级为任意精度 BigInt，覆盖词法/解析/求值/类型检查/标准库。
- **Logo assets**: SVG variants (glow/transparent), PNG sizes, and ICO exports. / Logo 资源包含 SVG（含光晕/透明）、多尺寸 PNG 与 ICO。
- **Documentation topics**: diagnostics, architecture, onboarding now available via `neve doc`. / `neve doc` 新增 diagnostics、architecture、onboarding 等主题。

### Improved / 改进
- **Docs overhaul**: All docs unified as bilingual (EN/中文) with consistent headers. / 文档整体统一为中英双语结构并统一视觉头部。
- **`neve doc` UX**: smarter topic matching, alias support, cleaner terminal rendering. / `neve doc` 支持别名与前缀匹配，渲染更干净。
- **Runtime safety**: conversions and indexing guard against overflow and negative indices. / 运行时更安全，转换与索引处理更稳健。
- **Release pipeline**: cross-platform artifacts produced by GitHub Actions. / Release 流水线使用 GitHub Actions 跨平台构建。

### Fixed / 修复
- **Numeric parsing**: integer parsing handles large values reliably. / 大整数解析更稳定。
- **Stdlib consistency**: map/set/list utilities align with BigInt semantics. / 标准库与 BigInt 语义一致。

## [0.7.0] - 2026-01-08

### Added / 新增
- **Frontend pipeline**: New `neve-frontend` crate for parse → HIR → typecheck analysis / **前端流水线**: 新增 `neve-frontend`，统一 parse → HIR → typecheck 分析
- **Docs**: Onboarding + diagnostics references, new `neve doc` topics / **文档**: 新增入门文档与诊断手册，并扩展 `neve doc` 主题
- **Tests**: Frontend diagnostics, module loader, formatter, and LSP symbol coverage / **测试**: 增加 frontend 诊断、模块加载、格式化器、LSP 符号覆盖
- **Stdlib imports**: `std.*` module overrides for AST evaluation / **标准库导入**: AST 求值支持 `std.*` 模块覆盖

### Improved / 改进
- **LSP**: Uses the frontend pipeline, fixes UTF-16 positions, adds diagnostic code links / **LSP**: 使用前端流水线，修正 UTF-16 位置，并附加错误码链接
- **Formatter**: Surfaces parser diagnostics for better error reporting / **格式化器**: 直接输出解析诊断，错误信息更清晰
- **Eval/Run**: Emits parse diagnostics for imported modules / **Eval/Run**: 导入模块解析出错时输出诊断
- **CLI eval/run/build/repl**: `import std.*` now resolves to the Rust stdlib modules / **CLI eval/run/build/repl**: `import std.*` 直接映射到 Rust 标准库模块
- **CLI check**: Reuses module loader parse diagnostics to avoid double parsing / **CLI check**: 复用模块加载的解析诊断，避免重复解析
- **Docs accuracy**: Spec/API/philosophy updated to match current syntax and stdlib / **文档准确性**: 修正文法/标准库/哲学文档与现状一致

## [0.6.4] - 2025-12-30

### Fixed / 修复
- **CI**: Fixed cross-compilation setup using `taiki-e/install-action` / **CI**: 使用 `taiki-e/install-action` 修复交叉编译设置
- **Formatting**: Fixed code formatting issues / **格式化**: 修复代码格式化问题

## [0.6.3] - 2025-12-30

### Improved / 改进
- **Documentation overhaul**: Updated docs/README.md as comprehensive documentation hub / **文档大改版**: 更新 docs/README.md 为综合文档中心
- **Architecture docs**: Added incremental compilation design section / **架构文档**: 添加增量编译设计章节
- **Changelog**: Synchronized with all v0.6.2 changes / **更新日志**: 同步所有 v0.6.2 变更

## [0.6.2] - 2025-12-30

### Added / 新增
- **Architecture documentation**: Comprehensive guide for contributors (`docs/contributor/architecture.md`) / **架构文档**: 为贡献者提供的全面指南 (`docs/contributor/architecture.md`)
- **CONTRIBUTING.md**: Bilingual contribution guidelines with setup instructions / **CONTRIBUTING.md**: 中英双语贡献指南，包含环境配置说明
- **Security audit in CI**: Added `cargo audit` for dependency vulnerability scanning / **CI 安全审计**: 添加 `cargo audit` 检测依赖漏洞
- **Incremental compilation cache**: ModuleCache with content-hash validation and dirty tracking / **增量编译缓存**: ModuleCache 支持内容哈希验证和脏标记跟踪
- **Cache query methods**: `has_content_changed()`, `get_cached_mtime()`, `get_cached_hash()` for fine-grained cache control / **缓存查询方法**: `has_content_changed()`、`get_cached_mtime()`、`get_cached_hash()` 提供细粒度缓存控制

### Improved / 改进
- **Release profile optimization**: LTO, strip, single codegen-unit for smaller binaries / **Release 配置优化**: LTO、符号剥离、单代码生成单元，生成更小的二进制文件
- **CI enhancement**: Clippy now checks all workspace crates, not just the main package / **CI 增强**: Clippy 现在检查所有 workspace crate，而不仅是主包
- **Stack safety**: Converted recursive directory operations to iterative (prevents stack overflow on deep directories) / **栈安全**: 将递归目录操作转换为迭代（防止深层目录栈溢出）
- **Memory optimization**: Pre-allocated capacity for `partition()`, `filter()`, `map_attrs()`, `filter_attrs()` operations / **内存优化**: 为 `partition()`、`filter()`、`map_attrs()`、`filter_attrs()` 操作预分配容量
- **Zero warnings**: Fixed all clippy warnings including unused fields and manual `div_ceil` implementations / **零警告**: 修复所有 clippy 警告，包括未使用字段和手动 `div_ceil` 实现

### Fixed / 修复
- **Super path resolution**: Fixed `super` import to correctly navigate module hierarchy (was skipping two levels instead of one in unit test) / **Super 路径解析**: 修复 `super` 导入以正确导航模块层级（单元测试中原本跳过了两级而非一级）
- **Type checker simplification**: Removed unused `name` and `generic_count` fields from StructInfo/EnumInfo/TypeAliasInfo / **类型检查器简化**: 移除 StructInfo/EnumInfo/TypeAliasInfo 中未使用的 `name` 和 `generic_count` 字段
- **MSRV declaration**: Added `rust-version = "1.85"` for Rust 2024 edition / **MSRV 声明**: 添加 `rust-version = "1.85"` 支持 Rust 2024 edition
- **Dev profile optimization**: Faster development builds with opt-level tuning / **开发配置优化**: 调整 opt-level 加快开发构建速度

## [0.6.1] - 2025-12-30

### Fixed / 修复
- **CI compatibility**: Resolved all clippy warnings for stable CI builds / **CI 兼容性**: 解决所有 clippy 警告，确保 CI 构建稳定
- **Code quality**: Fixed needless borrows, loop indexing patterns, and struct initialization / **代码质量**: 修复多余借用、循环索引模式和结构体初始化问题

## [0.6.0] - 2025-12-30

### Added / 新增
- **Tail Call Optimization (TCO)**: Recursive functions no longer cause stack overflow / **尾调用优化 (TCO)**: 递归函数不再导致栈溢出
- **NAR format implementation**: Complete Nix ARchive format support for content-addressed storage / **NAR 格式实现**: 完整的 Nix ARchive 格式支持，用于内容寻址存储
- **Build analytics module**: Dependency graph visualization with DOT format export / **构建分析模块**: 依赖图可视化，支持 DOT 格式导出
- **Enhanced CLI output**: Progress bars, spinners, tables, and colored output / **增强 CLI 输出**: 进度条、旋转器、表格和彩色输出
- **Security enhancements**: SecurityProfile for sandbox with seccomp, capabilities support / **安全增强**: 沙箱的 SecurityProfile，支持 seccomp、capabilities
- **Compression support**: gzip, xz, zstd for NAR archives / **压缩支持**: NAR 归档支持 gzip、xz、zstd

### Improved / 改进
- **Type error messages**: Better context and suggestions for type mismatches / **类型错误信息**: 类型不匹配时提供更好的上下文和建议
- **CLI commands**: All commands now use consistent output formatting / **CLI 命令**: 所有命令现在使用一致的输出格式
- **Binary units**: Size formatting now uses correct binary units (KiB/MiB/GiB) / **二进制单位**: 大小格式化现在使用正确的二进制单位 (KiB/MiB/GiB)
- **Zero warnings**: Codebase compiles with no warnings, all code serves its purpose / **零警告**: 代码库编译无警告，所有代码都发挥作用

### Fixed / 修复
- **NAR reader**: Fixed closing parenthesis handling in directory extraction / **NAR 读取器**: 修复目录提取时的闭括号处理
- **Cache tests**: Fixed permission issues with store tests / **缓存测试**: 修复存储测试的权限问题
- **Rust 2024**: Fixed pattern matching for new edition rules / **Rust 2024**: 修复新版本规则的模式匹配

## [0.5.0] - 2025-12-29

### Added / 新增
- **Bilingual source comments**: All source files now have English/Chinese comments / **双语源码注释**: 所有源文件现在都有中英文注释
- **Improved README**: Comprehensive installation guide with multiple methods / **改进的 README**: 包含多种安装方法的综合安装指南

### Improved / 改进
- **Code documentation**: Better inline documentation across all crates / **代码文档**: 所有 crate 的内联文档更完善

## [0.4.1] - 2025-12-29

### Added / 新增
- **Terminal Markdown rendering**: `neve doc` now renders with colors and styling / **终端 Markdown 渲染**: `neve doc` 现在有颜色和样式了
- **Windows one-line installer**: `irm .../install.ps1 | iex` / **Windows 一键安装**: `irm .../install.ps1 | iex`

### Improved / 改进
- Cross-platform install documentation with collapsible sections / 跨平台安装文档，用折叠面板分类
- Better code block and table rendering in docs / 代码块和表格渲染效果更好

## [0.4.0] - 2025-12-29

### Added / 新增
- **`neve doc` command**: Man-like documentation viewer with embedded docs / **`neve doc` 命令**: 类似 man 的文档查看器，文档直接嵌入二进制
- View any topic: `neve doc quickstart`, `neve doc api`, etc. / 查看任意主题: `neve doc quickstart`、`neve doc api` 等
- Uses pager (less/more) for comfortable reading / 自动用分页器 (less/more) 显示，看着舒服
- Available topics: quickstart, tutorial, spec, api, philosophy, install, changelog / 支持主题: quickstart、tutorial、spec、api、philosophy、install、changelog

### Improved / 改进
- **README redesign**: Cleaner layout and improved structure / **README 重新设计**: 更简洁的布局与结构优化
- **Documentation overhaul**: All docs restructured for clarity / **文档大改版**: 文档结构更清晰

## [0.3.1] - 2025-12-29

### Fixed / 修复
- **REPL interactivity**: Bare expressions now evaluate correctly (like Python) / **REPL 交互**: 直接输表达式现在能正常算了（跟 Python 一样）
- **Eval command**: Block expressions `{ let x = 1; x }` now work properly / **Eval 命令**: 块表达式 `{ let x = 1; x }` 现在能跑了
- **CI pipeline**: Fixed rustfmt/clippy component installation / **CI 流水线**: 修好了 rustfmt/clippy 组件安装问题
- **Cross-compilation**: aarch64-linux builds now use `cross` tool correctly / **交叉编译**: aarch64-linux 构建现在用 `cross` 工具能正常跑了

### Improved / 改进
- Expression handling in REPL with `prepare_repl_input()` preprocessing / REPL 里加了 `prepare_repl_input()` 预处理表达式
- CI workflow reliability across all platforms / CI 工作流在所有平台上都更稳定了

## [0.3.0] - 2025-12-29

## [0.2.0] - 2025-12-28

## [0.1.0] - 2024
