# n3v3-lsp: Language Server Protocol

## Architecture

```
Editor (VSCode / Helix / Neovim)
       │ LSP JSON-RPC
       ▼
┌──────────────────────────────────────────────┐
│  n3v3-lsp (Server)                            │
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
│  │  n3v3-frontend (analysis pipeline)      │  │
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

## LanguageServer Methods (26 LSP methods: 19 requests + 7 notifications)

| Method | Trait method | Data Source |
|--------|--------------|-------------|
| `initialize` | `initialize` | Server capabilities and initialization result |
| `initialized` | `initialized` | Client-ready notification and startup log |
| `shutdown` | `shutdown` | Server lifecycle |
| `textDocument/hover` | `hover` | `ModuleSemantics` and semantic hover maps |
| `textDocument/completion` | `completion` | Document symbols, stdlib, types, keywords |
| `completionItem/resolve` | `completion_resolve` | Standard-library and builtin documentation |
| `textDocument/signatureHelp` | `signature_help` | User-defined and builtin function signatures |
| `textDocument/definition` | `goto_definition` | `SymbolIndex` and resolved definition spans |
| `textDocument/references` | `references` | `SymbolIndex` reference spans |
| `textDocument/rename` | `rename` | `SymbolIndex` definition and reference spans |
| `textDocument/prepareRename` | `prepare_rename` | Rename-target validation |
| `textDocument/documentHighlight` | `document_highlight` | `SymbolIndex` reference spans |
| `textDocument/formatting` | `formatting` | `n3v3-fmt` integration |
| `textDocument/documentSymbol` | `document_symbol` | AST and `SymbolIndex` |
| `workspace/symbol` | `symbol` | Workspace symbol index |
| `textDocument/semanticTokens/full` | `semantic_tokens_full` | AST spans and lexer token kinds |
| `textDocument/inlayHint` | `inlay_hint` | `ModuleSemantics` inferred types |
| `textDocument/foldingRange` | `folding_range` | AST spans |
| `textDocument/codeAction` | `code_action` | Diagnostics → quick fixes |
| `textDocument/codeLens` | `code_lens` | Reference counts from `SymbolIndex` |
| `textDocument/didOpen` | `did_open` | Parse + frontend analysis |
| `textDocument/didChange` | `did_change` | Re-analysis of the document |
| `textDocument/didSave` | `did_save` | Re-analysis and diagnostic publication |
| `textDocument/didClose` | `did_close` | Document-store cleanup |
| `workspace/didChangeConfiguration` | `did_change_configuration` | Stub; settings require a server restart |
| `workspace/didChangeWatchedFiles` | `did_change_watched_files` | Stub; file events are not processed |

The `LanguageServer` implementation has 26 trait methods: 19 request
handlers and 7 notification handlers. `did_change_configuration` and
`did_change_watched_files` accept notifications but do not apply changes;
restart the server after configuration or watched-file changes.

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
$ n3v3 lsp --check
✓ LSP binary found
✓ JSON-RPC transport ready
✓ n3v3-frontend pipeline
✓ Document store
✓ Hover handler
✓ Completion handler
✓ Definition handler
✗ (7/7 checks passed)
```

## Editor Integration

### Helix
```bash
n3v3 setup helix  # One-shot: installs 6 query files + language config
```
Query files in `tree-sitter-n3v3/queries/`:
- `highlights.scm` — Fine-grained highlight scopes
- `injections.scm` — Language injection rules
- `locals.scm` — Local variable scoping
- `textobjects.scm` — Text objects and motions
- `folds.scm` — Code folding ranges
- `indents.scm` — Indentation rules

### VS Code
Extension in `editors/vscode/` — TextMate grammar + LSP client + publish script.

## Integration Points

| From | To | Data |
|------|----|------|
| n3v3-frontend | n3v3-lsp | `Module`, `ModuleSemantics`, `SymbolIndex`, diagnostics, type map |
| n3v3-fmt | n3v3-lsp | Formatted source text |
| n3v3-lsp | Editor | JSON-RPC responses |

## Key Files

| File | What |
|------|------|
| `crates/n3v3-lsp/src/lib.rs` | Server initialization + connection loop |
| `crates/n3v3-lsp/src/backend.rs` | All 26 `LanguageServer` trait methods (19 requests + 7 notifications) |
| `crates/n3v3-lsp/src/capabilities.rs` | Server capability registration |
| `crates/n3v3-lsp/src/document.rs` | Document store — open/changed/saved |
| `crates/n3v3-lsp/src/semantic_tokens.rs` | Semantic token encoding |
| `crates/n3v3-lsp/src/symbol_index.rs` | Workspace symbol indexing |
| `crates/n3v3-lsp/src/stdlib_completion/` | Standard library completions |
