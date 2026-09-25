# n3v3-diagnostic: Compiler Error System

## Crate: `n3v3-diagnostic`

Diagnostic and error reporting for n3v3 using ariadne. 55 diagnostic codes across 5 categories.

## Architecture

```
ErrorCode (codes.rs)       Diagnostic (diagnostic.rs)       emit() / explain() (lib.rs)
─────────────────────       ─────────────────────────       ─────────────────────────
55 diagnostic codes                severity + kind + code            ariadne Report → stderr
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

Type-checker warnings such as `duplicate definition of \`name\` shadows previous`
use `DiagnosticKind::Type` (not `DiagnosticKind::Parser`) and
`Severity::Warning`; warnings do not make `n3v3 check` fail.

## Public API

```rust
// Render a diagnostic to stderr (ariadne)
n3v3_diagnostic::emit(source: &str, filename: &str, diagnostic: &Diagnostic)

// Print extended explanation for an error code
n3v3_diagnostic::explain(code_str: &str) -> Result<(), String>

// Look up an error code from a string
n3v3_diagnostic::lookup_error_code(code_str: &str) -> Option<ErrorCode>
```

## Key Files

| File | Content |
|------|---------|
| `crates/n3v3-diagnostic/src/codes.rs` | Diagnostic code definitions and lookup mappings (55 diagnostic codes referenced by `scripts/counts.sh`) |
| `crates/n3v3-diagnostic/src/diagnostic.rs` | Diagnostic struct, Severity, DiagnosticKind, Label |
| `crates/n3v3-diagnostic/src/lib.rs` | emit(), explain() |
| `docs/reference/diagnostics.md` | Human-readable error code docs |

## Integration Points

- **Parser** (`n3v3-parser`): Uses `DiagnosticKind::Parser`, codes E0100-E0107
- **Typeck** (`n3v3-typeck`): Uses `DiagnosticKind::Type`, codes E0200-E0226; builders in `errors.rs`
- **Eval** (`n3v3-eval`): Bridge in `diagnostics.rs` converts `EvalError` → `Diagnostic` (E0300-E0306)
- **HIR** (`n3v3-hir`): Uses `DiagnosticKind::Module` for module-loading diagnostics (E0400-E0402)
- **CLI** (`n3v3-cli`): `n3v3 explain E####`, `emit_source_diagnostics()`, `emit_diagnostic_summary()`
- **LSP** (`n3v3-lsp`): Converts `Diagnostic` → `lsp_types::Diagnostic` in `publish_diagnostics()`

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
use n3v3_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label, emit};

let diag = Diagnostic::error(
    DiagnosticKind::Type,
    span,
    "mismatched types",
)
.with_code(ErrorCode::TypeMismatch)
.with_label(Label::new(span, "expected Int, found String"))
.with_help("use `toInt` to convert");

emit(source, "file.n3v3", &diag);
```
