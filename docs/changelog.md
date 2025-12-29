```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              NEVE CHANGELOG                                   ║
║                                更新日志                                        ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  [English]  #english   ──→  v0.4.1 / v0.4.0 / v0.3.1 / v0.3.0 / v0.2.0      │
│  [中文]     #chinese   ──→  v0.4.1 / v0.4.0 / v0.3.1 / v0.3.0 / v0.2.0      │
└─────────────────────────────────────────────────────────────────────────────┘
```

Based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

<a name="english"></a>

# English

> *What changed, when, and why.*

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
