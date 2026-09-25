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

## Implemented Methods (21)

| Method | Handler | Data Source |
|--------|---------|-------------|
| `textDocument/hover` | `handle_hover` | TypedHIR — type of expr/def at position |
| `textDocument/completion` | `handle_completion` | ModuleSemantics — locals, stdlib, types, keywords |
| `completionItem/resolve` | `handle_resolve` | Standard-library and builtin completion documentation |
| `textDocument/signatureHelp` | `handle_signature` | User-defined and builtin function signatures |
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

The table above lists 21 implemented LSP request and document-lifecycle
methods advertised by the server. The backend also accepts two protocol
notifications, `workspace/didChangeConfiguration` and
`workspace/didChangeWatchedFiles`; both are explicit stubs and are not counted
as implemented capabilities.

## Completion Architecture

Semantic tokens classify nested destructuring bindings from their original AST
identifier spans. Wildcards are intentionally omitted, while constructor
patterns retain their constructor/reference spans for definition navigation.

All positions produced by `semantic_tokens.rs` use LSP's default UTF-16 code
unit encoding: `offset_to_line_col` converts a byte offset to a UTF-16 column
and `utf16_length` converts byte spans to UTF-16 lengths, so an astral character
(emoji, 4 UTF-8 bytes / 2 UTF-16 units) shifts later tokens on the line by two
columns rather than by one (character counting) or four (byte counting).
Identifiers start with an ASCII letter or `_` but may continue with Unicode
alphanumerics (`café`), so a token's column width is measured in UTF-16 units
rather than assumed equal to its byte length.

Known limitation: a token that spans several lines (a raw multi-line string)
reports a length covering the whole span, which exceeds its first line. LSP
expects semantic tokens to stay within one line.

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

## Hover and Pattern Identity

Semantic hover traversal follows method receivers and every method argument,
including nested lambda bodies. Pattern indexing preserves one definition
identity for bindings introduced by an or-pattern, while constructor
references retain their resolved declaration spans.

## Type-Aware Completion

| Receiver Type | Representative methods | Example |
|---------------|------------------------|---------|
| `List<T>` | Mapping, filtering, folding, indexing | `.map`, `.filter`, `.fold`, `.head`, `.tail` |
| `String` | Splitting, trimming, case conversion, parsing | `.len`, `.split`, `.trim`, `.upper`, `.lower` |
| `Option<T>` | Mapping, unwrapping, filtering | `.map`, `.flatMap`, `.unwrap`, `.isSome` |
| `Result<T,E>` | Mapping, unwrapping, status checks | `.map`, `.flatMap`, `.unwrap`, `.isOk` |
| `Record` | Field access | `.fieldName` |

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
| `crates/neve-lsp/src/lib.rs` | Server initialization + connection loop |
| `crates/neve-lsp/src/backend.rs` | All 21 implemented method handlers |
| `crates/neve-lsp/src/capabilities.rs` | Server capability registration |
| `crates/neve-lsp/src/document.rs` | Document store — open/changed/saved |
| `crates/neve-lsp/src/semantic_tokens.rs` | Semantic token encoding |
| `crates/neve-lsp/src/symbol_index.rs` | Workspace symbol indexing |
| `crates/neve-lsp/src/stdlib_completion/` | Standard library completions |
