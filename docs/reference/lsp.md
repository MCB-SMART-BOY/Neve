<div align="center">

<img src="../../assets/logo.svg" width="120" alt="n3v3 logo">

<h1>n3v3 LSP — Language Server Protocol Implementation</h1>

<p><em>n3v3 LSP — 语言服务器协议实现</em></p>

<p>
  <strong><a href="../../README.md">Home</a></strong> ·
  <strong><a href="../README.md">Docs</a></strong>
</p>

</div>


## Overview

The n3v3 LSP server (`n3v3 lsp`) provides full IDE support for n3v3 source files (`.n3v3`). It implements the Language Server Protocol over stdio JSON-RPC and is compatible with any LSP-capable editor.

## Quick Start

```bash
# One-shot Helix setup
n3v3 setup helix

# Start the LSP server
n3v3 lsp

# Health check
n3v3 lsp --check
```

Open any `.n3v3` file in Helix — syntax highlighting, auto-completion, and diagnostics work out of the box.

## Supported LSP Methods — 26 LSP methods / 支持的 LSP 方法 — 26 个 LSP 方法

The `LanguageServer` implementation contains 19 request handlers and 7 notification handlers. The feature methods advertised by `server_capabilities()` are listed under “Implemented and declared”; lifecycle handlers, stubs, and unimplemented optional methods are separated below.
`LanguageServer` 实现包含 19 个请求 handler 与 7 个通知 handler。`server_capabilities()` 声明的功能列在“实现并声明”下；生命周期 handler、stub 与未实现的可选方法分开列出。

### Implemented and declared / 实现并声明

### Text Document Features / 文本文档功能

| Method | Status | Description |
|--------|--------|-------------|
| `textDocument/didOpen` | ✅ | Parse and analyze on open |
| `textDocument/didChange` | ✅ | Re-parse on change (full sync) |
| `textDocument/didSave` | ✅ | Re-parse on save |
| `textDocument/didClose` | ✅ | Clear diagnostics on close |
| `textDocument/hover` | ✅ | Type info + definition text + builtin docs |
| `textDocument/completion` | ✅ | Keywords, stdlib modules, types, methods, imports, document symbols |
| `textDocument/completionItem/resolve` | ✅ | Documentation for completion items |
| `textDocument/signatureHelp` | ✅ | User-defined and builtin function signatures |
| `textDocument/definition` | ✅ | Go-to-definition via scope-aware symbol index |
| `textDocument/references` | ✅ | Find all references with declaration toggle |
| `textDocument/documentHighlight` | ✅ | Read/write occurrence highlighting |
| `textDocument/rename` | ✅ | Batch rename with prepare support |
| `textDocument/prepareRename` | ✅ | Prepare rename validation |
| `textDocument/formatting` | ✅ | Format document via `n3v3-fmt` |
| `textDocument/documentSymbol` | ✅ | Hierarchical symbol view |
| `textDocument/semanticTokens/full` | ✅ | AST-based semantic tokens (10 types, 8 node kinds) |
| `textDocument/inlayHint` | ✅ | Type inference hints for let bindings and function returns |
| `textDocument/foldingRange` | ✅ | Code folding for functions, types, traits, impls |
| `textDocument/codeAction` | ✅ | Quick-fix diagnostics |
| `textDocument/codeLens` | ✅ | Reference counts on functions, types, and traits |

### Workspace Features / 工作区功能

| Method | Status | Description |
|--------|--------|-------------|
| `workspace/symbol` | ✅ | Search symbols across open documents |


### Protocol lifecycle handlers / 协议生命周期 handler

| Method | Status | Description |
|--------|--------|-------------|
| `initialize` | ✅ Implemented | Return server information and `server_capabilities()` |
| `shutdown` | ✅ Implemented | Complete the shutdown request |
| `initialized` | ✅ Implemented | Log successful server initialization |

### Protocol handlers but stub / 协议 handler 但为 stub

| Method | Status | Description |
|--------|--------|-------------|
| `workspace/didChangeConfiguration` | ⚠️ Stub | Accepts the event; restart the server for new settings to take effect |
| `workspace/didChangeWatchedFiles` | ⚠️ Stub | Accepts the event; restart the server because file changes are not re-analysed |

### Not implemented / 未实现

Optional methods not listed by `server_capabilities()` are not implemented or advertised. Examples include `textDocument/willSave`, `textDocument/willSaveWaitUntil`, `textDocument/rangeFormatting`, `textDocument/onTypeFormatting`, `textDocument/documentLink`, `textDocument/documentColor`, and `workspace/executeCommand`.
`server_capabilities()` 未列出的可选方法不会实现或声明。例如：`textDocument/willSave`、`textDocument/willSaveWaitUntil`、`textDocument/rangeFormatting`、`textDocument/onTypeFormatting`、`textDocument/documentLink`、`textDocument/documentColor` 与 `workspace/executeCommand`。

## Editor Integration

### Helix (Complete)

```bash
n3v3 setup helix
```

Installs:
- **Grammar**: `~/.config/helix/runtime/grammars/n3v3.so` (tree-sitter)
- **Queries**: 6 files in `~/.config/helix/runtime/queries/n3v3/`
  - `highlights.scm` — Syntax highlighting
  - `locals.scm` — Local variable scoping
  - `indents.scm` — Auto-indentation
  - `textobjects.scm` — Structural navigation
  - `injections.scm` — Language injection
  - `folds.scm` — Code folding
- **Config**: `~/.config/helix/languages.toml` — Language server + auto-format

Helix features enabled:
- ✅ Syntax highlighting (tree-sitter + LSP semantic tokens)
- ✅ Auto-completion (keywords, stdlib, types, methods, imports)
- ✅ Auto-format on save
- ✅ Code folding
- ✅ Structural text objects (`maf`, `mif`, etc.)
- ✅ Auto-indentation
- ✅ Inline type hints

### VS Code (Scaffolded)

Extension skeleton in `editors/vscode/`:
- `package.json` — Extension manifest with language configuration
- `language-configuration.json` — Comments, brackets, folding markers

### Sublime Text (Syntax Only)

Syntax definition in `editors/n3v3.sublime-syntax`.

## Completion Categories

The completion registry is assembled from the standard-library completion
modules in `crates/n3v3-lsp/src/stdlib_completion`; inventory counts are
intentionally not duplicated here.

| Category | Coverage | Example |
|----------|----------|---------|
| Keywords | Canonical and legacy parser keywords | `let`, `fn`, `if`, `match`, `trait`, `impl` |
| Standard library | Registry-backed module completions | `io.readFile`, `list.map`, `string.trim` |
| Types | Built-in and user-defined types | `Int`, `String`, `List`, `Option`, `Result`, `Stream` |
| Methods | Receiver-type-aware completions | `map`, `filter`, `split`, `unwrap`, `keys` |
| Import paths | Workspace-aware module paths | `.n3v3` modules |

### Type-Aware Method Completion

When typing `expr.`, only methods applicable to the expression's inferred type are shown:

| Receiver Type | Representative methods | Example |
|---------------|------------------------|---------|
| `List<T>` | Mapping, filtering, folding, indexing | `map`, `filter`, `fold`, `head`, `tail` |
| `String` | Splitting, trimming, case conversion, parsing | `split`, `trim`, `upper`, `replace`, `toInt` |
| `Option<T>` | Unwrapping, mapping, filtering | `unwrap`, `isSome`, `map`, `andThen` |
| `Result<T,E>` | Unwrapping, status checks, mapping | `unwrap`, `isOk`, `isErr`, `map`, `andThen` |
| `Record` | Field-oriented helpers | `keys`, `values`, `hasField` |

## Semantic Token Types

| Index | Type | AST Sources |
|-------|------|-------------|
| 0 | `keyword` | Lexer keywords |
| 1 | `variable` | Let bindings, use aliases, references |
| 2 | `function` | fn_def, trait items, impl items |
| 3 | `type` | type, trait, type alias names; enum variants |
| 4 | `string` | String/char/path literals |
| 5 | `number` | Int/float literals |
| 6 | `comment` | Reserved for future lexer comment tokens |
| 7 | `operator` | All operators and delimiters |
| 8 | `parameter` | Function/impl method parameters |
| 9 | `property` | Struct fields, field access, method calls |

## Architecture

```
n3v3-lsp (crate)
├── backend.rs     — LSP protocol handlers (2300+ lines)
├── capabilities.rs — Server capability declarations
├── document.rs    — Document model: parsing, analysis, hover maps
├── semantic_tokens.rs — Lexer and AST-based token generation
├── symbol_index.rs — Scope-aware symbol index (definitions + references)
└── stdlib_completion/ — Stdlib completion specs (9 modules)
```

The LSP server uses `n3v3-frontend` for the canonical analysis pipeline:
```
Source Text → Parser (AST) → Lowering (HIR) → Type Check → ModuleSemantics
                                                              ↓
                                              Diagnostics + SymbolIndex + HoverMaps
```

## Build & Test

```bash
# Build
cargo build -p n3v3

# Test LSP crate
cargo test -p n3v3-lsp          # Test the LSP crate

# Test LSP integration
cargo test --test lsp            # LSP integration tests

# Health check
cargo run -p n3v3 -- lsp --check  # Run the health check
```
