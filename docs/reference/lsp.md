# Neve LSP — Language Server Protocol Implementation

## Overview

The Neve LSP server (`neve lsp`) provides full IDE support for Neve source files (`.neve`). It implements the Language Server Protocol over stdio JSON-RPC and is compatible with any LSP-capable editor.

## Quick Start

```bash
# One-shot Helix setup
neve setup helix

# Start the LSP server
neve lsp

# Health check
neve lsp --check
```

Open any `.neve` file in Helix — syntax highlighting, auto-completion, and diagnostics work out of the box.

## Supported LSP Methods (20 total)

### Text Document Features

| Method | Status | Description |
|--------|--------|-------------|
| `textDocument/didOpen` | ✅ | Parse and analyze on open |
| `textDocument/didChange` | ✅ | Re-parse on change (full sync) |
| `textDocument/didSave` | ✅ | Re-parse on save |
| `textDocument/didClose` | ✅ | Clear diagnostics on close |
| `textDocument/hover` | ✅ | Type info + definition text + builtin docs (22 functions) |
| `textDocument/completion` | ✅ | Keywords, stdlib (81 fns), types (14), methods (54, type-aware), imports, document symbols |
| `textDocument/completionItem/resolve` | ✅ | Documentation for 24 functions |
| `textDocument/signatureHelp` | ✅ | User-defined + 60+ builtin function signatures |
| `textDocument/definition` | ✅ | Go-to-definition via scope-aware symbol index |
| `textDocument/references` | ✅ | Find all references with declaration toggle |
| `textDocument/documentHighlight` | ✅ | Read/write occurrence highlighting |
| `textDocument/rename` | ✅ | Batch rename with prepare support |
| `textDocument/formatting` | ✅ | Format document via `neve-fmt` |
| `textDocument/documentSymbol` | ✅ | Hierarchical symbol view |
| `textDocument/semanticTokens/full` | ✅ | AST-based semantic tokens (10 types, 8 node kinds) |
| `textDocument/inlayHint` | ✅ | Type inference hints for let bindings and function returns |
| `textDocument/foldingRange` | ✅ | Code folding for functions, types, traits, impls |
| `textDocument/codeAction` | ✅ | Quick-fix diagnostics |
| `textDocument/codeLens` | ✅ | Reference counts on functions, types, and traits |

### Workspace Features

| Method | Status | Description |
|--------|--------|-------------|
| `workspace/symbol` | ✅ | Search symbols across open documents |

## Editor Integration

### Helix (Complete)

```bash
neve setup helix
```

Installs:
- **Grammar**: `~/.config/helix/runtime/grammars/neve.so` (tree-sitter)
- **Queries**: 6 files in `~/.config/helix/runtime/queries/neve/`
  - `highlights.scm` — Syntax highlighting
  - `locals.scm` — Local variable scoping
  - `indents.scm` — Auto-indentation
  - `textobjects.scm` — Structural navigation
  - `injections.scm` — Language injection
  - `folds.scm` — Code folding
- **Config**: `~/.config/helix/languages.toml` — Language server + auto-format

Helix features enabled:
- ✅ Syntax highlighting (tree-sitter + LSP semantic tokens)
- ✅ Auto-completion (19 categories)
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

Syntax definition in `editors/neve.sublime-syntax`.

## Completion Categories

| Category | Count | Example |
|----------|-------|---------|
| Keywords | 13 | `let`, `fn`, `if`, `match`, `struct`, `enum`, `trait`, `impl` |
| Stdlib IO | 55 | `io.readFile`, `io.streamMap`, `io.cancel` |
| Stdlib List | 16 | `list.map`, `list.fold`, `list.zip` |
| Stdlib String | 12 | `string.split`, `string.trim` |
| Stdlib Math | 14 | `math.pi`, `math.sqrt` |
| Stdlib Path | 4 | `path.fromString`, `path.join` |
| Types | 14 | `Int`, `String`, `List`, `Option`, `Result`, `Stream` |
| Methods (type-aware) | 54 | `map`, `filter`, `split`, `unwrap`, `keys` |
| Import paths | dynamic | Scans workspace for `.neve` modules |

### Type-Aware Method Completion

When typing `expr.`, only methods applicable to the expression's inferred type are shown:

| Receiver Type | Methods | Example |
|---------------|---------|---------|
| `List<T>` | 32 | `map`, `filter`, `fold`, `head`, `tail`, `sort`, `sum`, `zip`, ... |
| `String` | 16 | `split`, `trim`, `upper`, `replace`, `lines`, `toInt`, ... |
| `Option<T>` | 11 | `unwrap`, `isSome`, `map`, `andThen`, `filter`, ... |
| `Result<T,E>` | 9 | `unwrap`, `isOk`, `isErr`, `map`, `andThen`, ... |
| `Record` | 3 | `keys`, `values`, `hasField` |

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
neve-lsp (crate)
├── backend.rs     — LSP protocol handlers (2300+ lines)
├── capabilities.rs — Server capability declarations
├── document.rs    — Document model: parsing, analysis, hover maps
├── semantic_tokens.rs — Lexer and AST-based token generation
├── symbol_index.rs — Scope-aware symbol index (definitions + references)
└── stdlib_completion/ — Stdlib completion specs (9 modules)
```

The LSP server uses `neve-frontend` for the canonical analysis pipeline:
```
Source Text → Parser (AST) → Lowering (HIR) → Type Check → ModuleSemantics
                                                              ↓
                                              Diagnostics + SymbolIndex + HoverMaps
```

## Build & Test

```bash
# Build
cargo build -p neve

# Test LSP crate
cargo test -p neve-lsp          # 13 unit tests

# Test LSP integration
cargo test --test lsp            # 142 integration tests

# Health check
cargo run -p neve -- lsp --check  # 7 automated checks
```
