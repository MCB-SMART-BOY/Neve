# Changelog

All notable changes to Neve will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-12-28

### 🎉 Major Features

#### REPL Environment Persistence
- **Interactive Programming Support**: REPL now maintains state across inputs, similar to Python/Haskell
- Variables and functions defined in REPL persist throughout the session
- Implemented using `Rc<RefCell<AstEnv>>` for mutable environment sharing

#### Module Re-export Fix
- Fixed infinite loop bug in `pub import` statements
- Added special handling for re-exports to detect circular dependencies
- Improved module loader with deferred symbol resolution

### ✨ New Features

#### Enhanced REPL Commands
- `:env` - Display all current bindings with visibility information
- `:load <file>` - Load and evaluate external `.neve` files
- `:type <expr>` - Type inference (placeholder for future implementation)
- `:clear` - Clear environment and start fresh
- `:help` - Improved help with usage tips

#### Multi-line Input Support
- Use `\` at end of line to continue on next line
- Dynamic prompt changes: `neve>` → `....>`
- Proper handling of multi-line function and let definitions

### 🔧 Improvements
- Added `eval_fn_def()` method for REPL function capture
- Improved environment cloning and binding extraction
- Better error messages with context
- Fixed all compilation errors and import conflicts

### 🐛 Bug Fixes
- **P0**: Module re-export infinite loop (tests/module_loading.rs:59)
- **P0**: REPL environment not persisting across inputs
- Import conflict resolution in ast_eval.rs

### 📦 Platform Support
- Cross-platform build support (Windows, macOS, Linux)
- GitHub Actions CI/CD for automated builds
- Platform-specific dependency handling (nix crate only on Unix)

### 🧪 Testing
- Enabled previously ignored module re-export test
- Network tests remain ignored (by design - require network access)
- All core functionality tests passing

### 🎯 Breaking Changes
None - this release is fully backward compatible with 0.1.0

---

## [0.1.0] - 2024

### Initial Release

#### Language Core (95% Complete)
- ✅ Complete lexer with logos-based tokenization
- ✅ Recursive descent parser (LL(1)) with error recovery
- ✅ Hindley-Milner type inference with trait support
- ✅ Tree-walking interpreter with lazy evaluation
- ✅ Module system with import/export

#### Standard Library
- 9 built-in modules: io, list, map, math, option, path, result, set, string

#### Toolchain (80% Complete)
- ✅ LSP server with semantic highlighting
- ✅ Code formatter
- ✅ Interactive REPL
- ✅ Comprehensive diagnostics

#### Package Management (60% Complete)
- ✅ Derivation model with hash verification
- ✅ Content-addressed store (BLAKE3)
- ✅ Sandboxed builder (Linux namespaces)
- ✅ Source fetcher (URL, Git, local)

#### System Configuration (40% Complete)
- ✅ Configuration framework
- ✅ Generation management
- Basic activation support

### Design Philosophy
- **Zero Ambiguity**: Every construct parses uniquely (LL(1))
- **Syntax Unification**: Consistent `fn` keyword, `#{}` for records
- **Pure Functional**: No side effects, reproducible builds
- **Modern Features**: Traits, pattern matching, type inference

[0.2.0]: https://github.com/MCB-SMART-BOY/neve/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MCB-SMART-BOY/neve/releases/tag/v0.1.0
