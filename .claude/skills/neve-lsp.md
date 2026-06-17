# neve-lsp: Language Server Protocol

## Architecture

```
Editor (VSCode / Helix / Neovim)
       │ LSP JSON-RPC
       ▼
┌──────────────────────────────────────────────┐
│  neve-lsp (Server)                            │
│  ┌────────────┐  ┌────────────┐              │
│  │ Connection │  │ Document   │              │
│  │ (transport)│  │ Store      │              │
│  └────────────┘  └────────────┘              │
│  ┌────────────────────────────────────────┐  │
│  │  Handlers                              │  │
│  │  hover / completion / goto def /       │  │
│  │  references / rename / format /        │  │
│  │  semantic tokens / inlay hints /       │  │
│  │  code lens / folding / code actions    │  │
│  └────────────────────────────────────────┘  │
│         │ uses                                │
│         ▼                                     │
│  ┌────────────────────────────────────────┐  │
│  │  neve-frontend (analysis pipeline)      │  │
│  │  parse → HIR → typeck → type map        │  │
│  └────────────────────────────────────────┘  │
└──────────────────────────────────────────────┘
       │
       ▼
  LSP Client responses:
    HoverContents, CompletionList,
    GotoDefinitionResponse, SemanticTokens, ...
```

## Document Lifecycle

```
didOpen  →  parse → HIR → typeck → cache results
    │
didChange →  re-parse (incremental) → re-typeck → update cache
    │
didSave  →  (same as didChange, plus diagnostic publish)
    │
didClose →  remove from document store
```

## Implemented Methods (20)

| Method | Handler | Data Source |
|--------|---------|-------------|
| `textDocument/hover` | `handle_hover` | TypedHIR — type of expr/def at position |
| `textDocument/completion` | `handle_completion` | ModuleSemantics — locals, stdlib, types, keywords |
| `completionItem/resolve` | `handle_resolve` | Docs database — 77 function docs |
| `textDocument/signatureHelp` | `handle_signature` | 80 builtin sigs + user fn sigs via AST |
| `textDocument/definition` | `handle_goto_def` | DefTable — resolved DefId → span |
| `textDocument/references` | `handle_references` | DefTable — all uses of a DefId |
| `textDocument/rename` | `handle_rename` | DefTable — rename across workspace |
| `textDocument/prepareRename` | `handle_prepare_rename` | DefTable — validate rename target |
| `textDocument/documentHighlight` | `handle_highlight` | DefTable — highlight all uses |
| `textDocument/formatting` | `handle_format` | neve-fmt integration |
| `textDocument/documentSymbol` | `handle_symbols` | AST — module structure tree |
| `workspace/symbol` | `handle_workspace_symbol` | Cross-module DefTable |
| `textDocument/semanticTokens/full` | `handle_tokens` | AST-based + lexer fallback |
| `textDocument/inlayHint` | `handle_inlay` | TypedHIR — type annotations |
| `textDocument/foldingRange` | `handle_folding` | AST — block/record/match ranges |
| `textDocument/codeAction` | `handle_code_actions` | Diagnostics → quick fixes |
| `textDocument/codeLens` | `handle_code_lens` | Reference counts on fn/struct/trait |
| `textDocument/didOpen` | `handle_open` | Parse + typeck |
| `textDocument/didChange` | `handle_change` | Incremental re-parse |
| `textDocument/didSave` | `handle_save` | Publish diagnostics |
| `textDocument/didClose` | `handle_close` | Cleanup |

## Completion Architecture

```
User types:  dat
              │
              ▼
┌─────────────────────────────────────┐
│ 1. Local scope lookup               │
│    dat → data (DefId: local var)    │
│ 2. Stdlib lookup                    │
│    dat → string.split, list.filter  │
│ 3. Type-aware filtering             │
│    Receiver type? → method filter   │
│ 4. Keyword/type fallback            │
│    dat → (none)                     │
└─────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────┐
│ Scoring                             │
│ Exact > Prefix > Contains           │
│ Local > Stdlib > Type > Keyword     │
└─────────────────────────────────────┘
              │
              ▼
       CompletionList (sorted)
```

## Type-Aware Completion (54 methods, 5 receiver types)

| Receiver Type | Method Count | Example |
|---------------|-------------|---------|
| `List<T>` | 32 | `.map`, `.filter`, `.fold`, `.head`, `.tail`... |
| `String` | 16 | `.len`, `.split`, `.trim`, `.upper`, `.lower`... |
| `Option<T>` | 11 | `.map`, `.flatMap`, `.unwrap`, `.isSome`... |
| `Result<T,E>` | 9 | `.map`, `.flatMap`, `.unwrap`, `.isOk`... |
| `Record` | 3 | Field access via `.fieldName` |

## CodeLens — Reference Counts

```
│  3 references                                 │  ← CodeLens above fn/struct/trait
│  fn add(a: Int, b: Int) -> Int = a + b        │
│                                                │
│  let x = add(1, 2)  // ← reference 1           │
│  let y = add(3, 4)  // ← reference 2           │
│  let z = add(5, 6)  // ← reference 3           │
```

## Health Check

```bash
$ neve lsp --check
✓ LSP binary found
✓ JSON-RPC transport ready
✓ neve-frontend pipeline
✓ Document store
✓ Hover handler
✓ Completion handler
✓ Definition handler
✗ (7/7 checks passed)
```

## Editor Integration

### Helix
```bash
neve setup helix  # One-shot: installs 6 query files + language config
```
Query files in `editors/tree-sitter-neve/queries/`:
- `highlights.scm` — 22 fine-grained scopes
- `injections.scm` — Language injection rules
- `locals.scm` — Local variable scoping
- `motions.scm` — Text objects and motions
- `folds.scm` — Code folding ranges
- `indents.scm` — Indentation rules

### VS Code
Extension in `editors/vscode/` — TextMate grammar + LSP client + publish script.

## Integration Points

| From | To | Data |
|------|----|------|
| neve-frontend | neve-lsp | `TypedModule`, `DefTable`, diagnostics, type map |
| neve-fmt | neve-lsp | Formatted source text |
| neve-lsp | Editor | JSON-RPC responses |

## Key Files

| File | What |
|------|------|
| `lsp/src/lib.rs` | Server initialization + connection loop |
| `lsp/src/backend.rs` | All 23 LSP method handlers |
| `lsp/src/capabilities.rs` | Server capability registration |
| `lsp/src/document.rs` | Document store — open/changed/saved |
| `lsp/src/semantic_tokens.rs` | Semantic token encoding |
| `lsp/src/symbol_index.rs` | Workspace symbol indexing |
| `lsp/src/stdlib_completion/` | Standard library completions |
