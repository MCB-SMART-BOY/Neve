# Contributing to Neve / 贡献指南

Thank you for your interest in contributing to Neve! This document provides guidelines and instructions for contributing.

感谢您对 Neve 项目的关注！本文档提供贡献指南和说明。

---

## Table of Contents / 目录

- [Code of Conduct / 行为准则](#code-of-conduct--行为准则)
- [Getting Started / 开始贡献](#getting-started--开始贡献)
- [Development Setup / 开发环境](#development-setup--开发环境)
- [Project Structure / 项目结构](#project-structure--项目结构)
- [Code Style / 代码风格](#code-style--代码风格)
- [Pull Request Process / PR 流程](#pull-request-process--pr-流程)
- [Reporting Issues / 报告问题](#reporting-issues--报告问题)

---

## Code of Conduct / 行为准则

Be respectful, inclusive, and constructive. We welcome contributors of all skill levels.

请保持尊重、包容和建设性态度。我们欢迎各种技能水平的贡献者。

---

## Getting Started / 开始贡献

### Prerequisites / 前置要求

- **Rust nightly** (1.85+) - Required for Rust 2024 edition
- **Git** - Version control
- **Linux/macOS** - For full functionality (Windows supports language features only)

```bash
# Install Rust nightly / 安装 Rust nightly
rustup install nightly
rustup default nightly

# Verify installation / 验证安装
rustc --version  # Should show nightly-2024-xx-xx or later
```

---

## Development Setup / 开发环境

```bash
# Clone the repository / 克隆仓库
git clone https://github.com/MCB-SMART-BOY/Neve.git
cd Neve

# Build the project / 构建项目
cargo build

# Run tests / 运行测试
cargo test

# Run the CLI / 运行 CLI
cargo run -p neve -- --help

# Format code / 格式化代码
cargo fmt

# Run lints / 运行 lint
cargo clippy --workspace -- -D warnings
```

---

## Project Structure / 项目结构

```
Neve/
├── neve-cli/              # CLI application / CLI 应用
├── crates/
│   ├── neve-lexer/        # Tokenizer / 词法分析器
│   ├── neve-parser/       # Parser / 语法分析器
│   ├── neve-syntax/       # AST definitions / AST 定义
│   ├── neve-hir/          # High-level IR / 高级中间表示
│   ├── neve-typeck/       # Type checker / 类型检查器
│   ├── neve-eval/         # Interpreter / 解释器
│   ├── neve-std/          # Standard library / 标准库
│   ├── neve-store/        # Content-addressed store / 内容寻址存储
│   ├── neve-fetch/        # Source fetching / 源码获取
│   ├── neve-builder/      # Build system / 构建系统
│   ├── neve-config/       # System configuration / 系统配置
│   ├── neve-lsp/          # Language server / 语言服务器
│   ├── neve-fmt/          # Code formatter / 代码格式化
│   ├── neve-diagnostic/   # Error reporting / 错误报告
│   ├── neve-common/       # Shared utilities / 共享工具
│   └── neve-derive/       # Proc macros / 过程宏
├── docs/                  # Documentation / 文档
├── tests/                 # Integration tests / 集成测试
└── examples/              # Example code / 示例代码
```

### Data Flow / 数据流

```
Source Code → Lexer → Parser → HIR → TypeChecker → Evaluator
    ↓           ↓        ↓       ↓         ↓           ↓
  .neve      Tokens    AST     HIR    Typed HIR    Value
```

---

## Code Style / 代码风格

### General Guidelines / 通用指南

1. **Run formatters before committing / 提交前运行格式化**
   ```bash
   cargo fmt
   cargo clippy --workspace -- -D warnings
   ```

2. **Write bilingual comments / 编写双语注释**
   ```rust
   /// Parse an expression.
   /// 解析表达式。
   fn parse_expr(&mut self) -> Result<Expr> { ... }
   ```

3. **Prefer `?` over `unwrap()` / 优先使用 `?` 而非 `unwrap()`**
   ```rust
   // Bad / 不推荐
   let value = map.get("key").unwrap();
   
   // Good / 推荐
   let value = map.get("key").ok_or(Error::KeyNotFound)?;
   ```

4. **Use descriptive names / 使用描述性名称**
   ```rust
   // Bad / 不推荐
   let x = parse(s)?;
   
   // Good / 推荐
   let expression = parse_expression(source_code)?;
   ```

### Commit Messages / 提交信息

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add pattern matching support
fix: resolve infinite loop in module loading
docs: update installation guide
refactor: simplify type unification
test: add unit tests for lexer
chore: update dependencies
```

---

## Pull Request Process / PR 流程

1. **Fork and branch / Fork 并创建分支**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make changes / 修改代码**
   - Write tests for new functionality
   - Update documentation if needed
   - Ensure all tests pass

3. **Submit PR / 提交 PR**
   - Describe what changes you made and why
   - Reference any related issues
   - Wait for CI to pass

4. **Code review / 代码审查**
   - Address feedback promptly
   - Keep discussions constructive

---

## Reporting Issues / 报告问题

### Bug Reports / Bug 报告

Include:
- Neve version (`neve --version`)
- Operating system
- Minimal reproduction steps
- Expected vs actual behavior

### Feature Requests / 功能请求

Include:
- Use case description
- Proposed solution (if any)
- Alternatives considered

---

## License / 许可证

By contributing, you agree that your contributions will be licensed under the MPL-2.0 license.

通过贡献，您同意您的贡献将在 MPL-2.0 许可证下发布。

---

## Questions? / 有问题？

- Open an issue on GitHub
- Check existing issues and discussions

Thank you for contributing! / 感谢您的贡献！
