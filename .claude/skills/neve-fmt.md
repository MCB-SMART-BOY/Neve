# neve-fmt: Formatter

## Crate: `neve-fmt`

Pretty-printer for Neve source code.

## Architecture

```
Source → Parser → AST → Formatter → Printer → String
```

The formatter walks the AST and produces formatted output through a `Printer` with configurable width.

## Key Types

- **`format(source)`** — Main entry point. Returns `Result<String, FormatError>`.
- **`FormatError`** — `Internal(String)` variant for recoverable errors.
- **`Printer`** — Width-aware output buffer. Key methods:
  - `would_exceed_width(remaining) -> bool` — Line wrapping decision
  - `max_width` — Configurable line width

## Key Files

| File | Content |
|------|---------|
| `crates/neve-fmt/src/format.rs` | Main formatting logic, expression/item formatting |
| `crates/neve-fmt/src/lib.rs` | Public API, `FormatError` |
| `crates/neve-fmt/src/printer.rs` | `Printer` struct, `would_exceed_width()` |
| `tests/fmt.rs` | Formatter tests (idempotency, syntax) |

## Integration Points

- **CLI** (`neve-cli`): `neve fmt file/check/dir` commands
- **Parser** (`neve-parser`): Formatter consumes parsed AST
- All CLI fmt invocations go through `neve-fmt::format()`

## CLI Commands

```bash
neve fmt file path/to/file.neve         # Format a file
neve fmt file path/to/file.neve --write # Format and write back
neve fmt check path/to/file.neve        # Check if formatted (exit 1 if not)
neve fmt dir .                          # Format all .neve files in directory
```

## Testing

```bash
cargo test -p neve-fmt
cargo test --test fmt
```

## Gotchas

- `format()` returns `Result`, not `String`. The old `debug_assert!`-based approach was replaced in audit fix M18.
- `would_exceed_width()` is used for basic line wrapping (H11).
- Formatter drops comments (known limitation, see audit finding C6).
