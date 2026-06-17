# neve-diagnostic: Compiler Error System

## Crate: `neve-diagnostic`

Diagnostic and error reporting for Neve using ariadne. 53 error codes across 5 categories.

## Architecture

```
ErrorCode (codes.rs)       Diagnostic (diagnostic.rs)       emit() / explain() (lib.rs)
─────────────────────       ─────────────────────────       ─────────────────────────
53 variants                severity + kind + code            ariadne Report → stderr
extended_explanation()     message + span + labels           lookup_error_code()
suggestion()               notes + help
doc_url()
```

## Error Code Ranges

| Range | Category | Count |
|-------|----------|-------|
| E0001-E0005 | Lexer | 5 |
| E0100-E0107 | Parser | 8 |
| E0200-E0226 | Type | 27 |
| E0300-E0306 | Eval | 7 |
| E0400-E0402 | Module | 3 |

## Key Types

- **`Diagnostic`** — Universal error carrier. Builder pattern: `.error(kind, span, msg)` / `.warning(kind, span, msg)` → `.with_code()` → `.with_label()` → `.with_note()` → `.with_help()`
- **`ErrorCode`** — Enum with `as_str()` ("E0200"), `description()`, `suggestion()`, `extended_explanation()`, `doc_url()`
- **`Severity`** — Error | Warning | Note
- **`DiagnosticKind`** — Lexer | Parser | Type | Eval | Module
- **`Label`** — Span + message for source annotations

## Public API

```rust
// Render a diagnostic to stderr (ariadne)
neve_diagnostic::emit(source: &str, filename: &str, diagnostic: &Diagnostic)

// Print extended explanation for an error code
neve_diagnostic::explain(code_str: &str) -> Result<(), String>

// Look up an error code from a string
neve_diagnostic::lookup_error_code(code_str: &str) -> Option<ErrorCode>
```

## Key Files

| File | Content |
|------|---------|
| `crates/neve-diagnostic/src/codes.rs` | ErrorCode enum (53 variants), lookup_error_code() |
| `crates/neve-diagnostic/src/diagnostic.rs` | Diagnostic struct, Severity, DiagnosticKind, Label |
| `crates/neve-diagnostic/src/lib.rs` | emit(), explain() |
| `docs/reference/diagnostics.md` | Human-readable error code docs |

## Integration Points

- **Parser** (`neve-parser`): Uses `DiagnosticKind::Parser`, codes E0100-E0107
- **Typeck** (`neve-typeck`): Uses `DiagnosticKind::Type`, codes E0200-E0226; builders in `errors.rs`
- **Eval** (`neve-eval`): Bridge in `diagnostics.rs` converts `EvalError` → `Diagnostic` (E0300-E0306)
- **HIR** (`neve-hir`): Uses `DiagnosticKind::Module` for module-loading diagnostics (E0400-E0402)
- **CLI** (`neve-cli`): `neve explain E####`, `emit_source_diagnostics()`, `emit_diagnostic_summary()`
- **LSP** (`neve-lsp`): Converts `Diagnostic` → `lsp_types::Diagnostic` in `publish_diagnostics()`

## How to Add a New Error Code

1. Add variant to `ErrorCode` enum in `codes.rs`
2. Add `as_str()` mapping (e.g. `"E0XXX"`)
3. Add `description()` text
4. Add `suggestion()` if applicable
5. Add `extended_explanation()` if the code needs a detailed `--explain` page
6. Add `lookup_error_code()` arm
7. Add documentation anchor in `docs/reference/diagnostics.md`

## Usage Example

```rust
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label, emit};

let diag = Diagnostic::error(
    DiagnosticKind::Type,
    span,
    "mismatched types",
)
.with_code(ErrorCode::TypeMismatch)
.with_label(Label::new(span, "expected Int, found String"))
.with_help("use `toInt` to convert");

emit(source, "file.neve", &diag);
```
