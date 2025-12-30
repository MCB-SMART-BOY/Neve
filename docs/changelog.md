```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              NEVE CHANGELOG                                   ║
║                                更新日志                                        ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  v0.6.2 / v0.6.1 / v0.6.0 / v0.5.0 / v0.4.x      │
│  [中文]     #chinese   ──→  v0.6.2 / v0.6.1 / v0.6.0 / v0.5.0 / v0.4.x      │
└─────────────────────────────────────────────────────────────────────────────┘
```

Based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

<a name="english"></a>

# English

> *What changed, when, and why.*

## [0.6.2] - 2025-12-30

### Added
- **Architecture documentation**: Comprehensive guide for contributors (`docs/architecture.md`)
- **CONTRIBUTING.md**: Bilingual contribution guidelines with setup instructions
- **Security audit in CI**: Added `cargo audit` for dependency vulnerability scanning

### Improved
- **Release profile optimization**: LTO, strip, single codegen-unit for smaller binaries
- **CI enhancement**: Clippy now checks all workspace crates, not just the main package
- **Stack safety**: Converted recursive directory operations to iterative (prevents stack overflow on deep directories)

### Developer Experience
- **MSRV declaration**: Added `rust-version = "1.85"` for Rust 2024 edition
- **Dev profile optimization**: Faster development builds with opt-level tuning

## [0.6.1] - 2025-12-30

### Fixed
- **CI compatibility**: Resolved all clippy warnings for stable CI builds
- **Code quality**: Fixed needless borrows, loop indexing patterns, and struct initialization

## [0.6.0] - 2025-12-30

### Added
- **Tail Call Optimization (TCO)**: Recursive functions no longer cause stack overflow
- **NAR format implementation**: Complete Nix ARchive format support for content-addressed storage
- **Build analytics module**: Dependency graph visualization with DOT format export
- **Enhanced CLI output**: Progress bars, spinners, tables, and colored output
- **Security enhancements**: SecurityProfile for sandbox with seccomp, capabilities support
- **Compression support**: gzip, xz, zstd for NAR archives

### Improved
- **Type error messages**: Better context and suggestions for type mismatches
- **CLI commands**: All commands now use consistent output formatting
- **Binary units**: Size formatting now uses correct binary units (KiB/MiB/GiB)
- **Zero warnings**: Codebase compiles with no warnings, all code serves its purpose

### Fixed
- **NAR reader**: Fixed closing parenthesis handling in directory extraction
- **Cache tests**: Fixed permission issues with store tests
- **Rust 2024**: Fixed pattern matching for new edition rules

## [0.5.0] - 2025-12-29

### Added
- **Bilingual source comments**: All source files now have English/Chinese comments
- **Improved README**: Comprehensive installation guide with multiple methods

### Improved
- **Code documentation**: Better inline documentation across all crates

## [0.4.1] - 2025-12-29

### Added
- **Terminal Markdown rendering**: `neve doc` now renders with colors and styling
- **Windows one-line installer**: `irm .../install.ps1 | iex`

### Improved
- Cross-platform install documentation with collapsible sections
- Better code block and table rendering in docs

## [0.4.0] - 2025-12-29

### Added
- **`neve doc` command**: Man-like documentation viewer with embedded docs
  - View any topic: `neve doc quickstart`, `neve doc api`, etc.
  - Language filter: `--en` for English only, `--zh` for Chinese only
  - Uses pager (less/more) for comfortable reading
  - Available topics: quickstart, tutorial, spec, api, philosophy, install, changelog

### Improved
- **README redesign**: Cleaner layout with working anchor links for language switching
- **Documentation overhaul**: All docs restructured with English first, Chinese second

## [0.3.1] - 2025-12-29

### Fixed
- **REPL interactivity**: Bare expressions now evaluate correctly (like Python)
- **Eval command**: Block expressions `{ let x = 1; x }` now work properly
- **CI pipeline**: Fixed rustfmt/clippy component installation
- **Cross-compilation**: aarch64-linux builds now use `cross` tool correctly

### Improved
- Expression handling in REPL with `prepare_repl_input()` preprocessing
- CI workflow reliability across all platforms

## [0.3.0] - 2025-12-29

### Cross-Platform Support
- **Windows/macOS**: Full language support (eval, repl, check, fmt, lsp)
- **Docker build backend**: `--backend docker` option for sandbox builds on non-Linux
- **Platform-aware CLI**: `neve info --platform` shows platform capabilities

### Platform Matrix

| Feature | Linux | macOS | Windows |
|---------|-------|-------|---------|
| Language Core | ✅ | ✅ | ✅ |
| Native Sandbox Build | ✅ | ❌ | ❌ |
| Docker Build | ✅ | ✅ | ✅ |
| System Config | ✅ | ❌ | ❌ |

## [0.2.0] - 2025-12-28

### Major Features
- **REPL environment persistence**: Variables and functions persist across inputs
- **Module re-export fix**: Fixed infinite loop bug in `pub import`

### REPL Enhancements
- `:env` - Display current bindings
- `:load <file>` - Load external file
- `:clear` - Clear environment
- Multi-line input support (trailing `\`)

### Bug Fixes
- Module re-export infinite loop
- Import conflict resolution

## [0.1.0] - 2024

### Initial Release

#### Language Core (95%)
- Complete lexer (logos)
- Recursive descent parser (LL(1)) + error recovery
- Hindley-Milner type inference + Trait support
- Tree-walking interpreter + lazy evaluation
- Module system

#### Standard Library
- 9 modules: io, list, map, math, option, path, result, set, string

#### Toolchain (80%)
- LSP server
- Code formatter
- Interactive REPL
- Diagnostic system

#### Package Manager (60%)
- Derivation model + hash verification
- Content-addressed storage (BLAKE3)
- Sandbox builder (Linux namespaces)
- Source fetching (URL, Git, local)

#### System Configuration (40%)
- Configuration framework
- Generation management

---

<a name="chinese"></a>

# 中文

> 改了啥、啥时候改的、为啥改。

## [0.6.2] - 2025-12-30

### 新功能
- **架构文档**: 为贡献者提供的全面指南 (`docs/architecture.md`)
- **CONTRIBUTING.md**: 中英双语贡献指南，包含环境配置说明
- **CI 安全审计**: 添加 `cargo audit` 检测依赖漏洞

### 改进
- **Release 配置优化**: LTO、符号剥离、单代码生成单元，生成更小的二进制文件
- **CI 增强**: Clippy 现在检查所有 workspace crate，而不仅是主包
- **栈安全**: 将递归目录操作转换为迭代（防止深层目录栈溢出）

### 开发体验
- **MSRV 声明**: 添加 `rust-version = "1.85"` 支持 Rust 2024 edition
- **开发配置优化**: 调整 opt-level 加快开发构建速度

## [0.6.1] - 2025-12-30

### 修复
- **CI 兼容性**: 解决所有 clippy 警告，确保 CI 构建稳定
- **代码质量**: 修复多余借用、循环索引模式和结构体初始化问题

## [0.6.0] - 2025-12-30

### 新功能
- **尾调用优化 (TCO)**: 递归函数不再导致栈溢出
- **NAR 格式实现**: 完整的 Nix ARchive 格式支持，用于内容寻址存储
- **构建分析模块**: 依赖图可视化，支持 DOT 格式导出
- **增强 CLI 输出**: 进度条、旋转器、表格和彩色输出
- **安全增强**: 沙箱的 SecurityProfile，支持 seccomp、capabilities
- **压缩支持**: NAR 归档支持 gzip、xz、zstd

### 改进
- **类型错误信息**: 类型不匹配时提供更好的上下文和建议
- **CLI 命令**: 所有命令现在使用一致的输出格式
- **二进制单位**: 大小格式化现在使用正确的二进制单位 (KiB/MiB/GiB)
- **零警告**: 代码库编译无警告，所有代码都发挥作用

### 修复
- **NAR 读取器**: 修复目录提取时的闭括号处理
- **缓存测试**: 修复存储测试的权限问题
- **Rust 2024**: 修复新版本规则的模式匹配

## [0.5.0] - 2025-12-29

### 新功能
- **双语源码注释**: 所有源文件现在都有中英文注释
- **改进的 README**: 包含多种安装方法的综合安装指南

### 改进
- **代码文档**: 所有 crate 的内联文档更完善

## [0.4.1] - 2025-12-29

### 新功能
- **终端 Markdown 渲染**: `neve doc` 现在有颜色和样式了
- **Windows 一键安装**: `irm .../install.ps1 | iex`

### 改进
- 跨平台安装文档，用折叠面板分类
- 代码块和表格渲染效果更好

## [0.4.0] - 2025-12-29

### 新功能
- **`neve doc` 命令**: 类似 man 的文档查看器，文档直接嵌入二进制
  - 查看任意主题: `neve doc quickstart`、`neve doc api` 等
  - 语言过滤: `--en` 只看英文，`--zh` 只看中文
  - 自动用分页器 (less/more) 显示，看着舒服
  - 支持主题: quickstart、tutorial、spec、api、philosophy、install、changelog

### 改进
- **README 重新设计**: 更简洁的布局，中英文跳转链接真正可用了
- **文档大改版**: 所有文档重新组织，英文在上中文在下

## [0.3.1] - 2025-12-29

### 修复
- **REPL 交互**: 直接输表达式现在能正常算了（跟 Python 一样）
- **Eval 命令**: 块表达式 `{ let x = 1; x }` 现在能跑了
- **CI 流水线**: 修好了 rustfmt/clippy 组件安装问题
- **交叉编译**: aarch64-linux 构建现在用 `cross` 工具能正常跑了

### 改进
- REPL 里加了 `prepare_repl_input()` 预处理表达式
- CI 工作流在所有平台上都更稳定了

## [0.3.0] - 2025-12-29

### 跨平台支持
- **Windows/macOS**: 语言功能全都能用（eval、repl、check、fmt、lsp）
- **Docker 构建后端**: 加了 `--backend docker` 选项，非 Linux 平台也能沙箱构建
- **平台感知 CLI**: `neve info --platform` 能显示当前平台支持啥

### 平台功能表

| 功能 | Linux | macOS | Windows |
|------|-------|-------|---------|
| 语言核心 | ✅ | ✅ | ✅ |
| 原生沙箱构建 | ✅ | ❌ | ❌ |
| Docker 构建 | ✅ | ✅ | ✅ |
| 系统配置 | ✅ | ❌ | ❌ |

## [0.2.0] - 2025-12-28

### 主要功能
- **REPL 环境持久化**: 变量和函数在会话里一直保持
- **模块重导出修复**: 修好了 `pub import` 的死循环 bug

### REPL 增强
- `:env` - 显示当前绑定的东西
- `:load <file>` - 加载外部文件
- `:clear` - 清空环境
- 支持多行输入（行尾加 `\`）

### Bug 修复
- 模块重导出死循环
- Import 冲突解析

## [0.1.0] - 2024

### 初始版本

#### 语言核心 (95%)
- 完整的词法分析器 (logos)
- 递归下降解析器 (LL(1)) + 错误恢复
- Hindley-Milner 类型推导 + Trait 支持
- 树遍历解释器 + 惰性求值
- 模块系统

#### 标准库
- 9 个模块: io、list、map、math、option、path、result、set、string

#### 工具链 (80%)
- LSP 服务器
- 代码格式化器
- 交互式 REPL
- 诊断系统

#### 包管理 (60%)
- Derivation 模型 + 哈希校验
- 内容寻址存储 (BLAKE3)
- 沙箱构建器 (Linux 命名空间)
- 源码获取 (URL、Git、本地)

#### 系统配置 (40%)
- 配置框架
- 代际管理

---

[0.6.2]: https://github.com/MCB-SMART-BOY/neve/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/MCB-SMART-BOY/neve/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/MCB-SMART-BOY/neve/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/MCB-SMART-BOY/neve/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MCB-SMART-BOY/neve/releases/tag/v0.1.0

---

<div align="center">

```
═══════════════════════════════════════════════════════════════════════════════
                     Every version tells a story.
═══════════════════════════════════════════════════════════════════════════════
```

</div>
