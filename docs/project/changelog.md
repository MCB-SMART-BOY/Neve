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

## [Unreleased] / 未发布

### Added / 新增
- (nothing yet)

### Improved / 改进
- (nothing yet)

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
