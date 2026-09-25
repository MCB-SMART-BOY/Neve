# n3v3-fmt: Formatter

## Crate: `n3v3-fmt`

Pretty-printer for n3v3 source code.

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
| `crates/n3v3-fmt/src/format.rs` | Main formatting logic, expression/item formatting |
| `crates/n3v3-fmt/src/lib.rs` | Public API, `FormatError` |
| `crates/n3v3-fmt/src/printer.rs` | `Printer` struct, `would_exceed_width()` |
| `tests/fmt.rs` | Formatter tests (idempotency, syntax) |

## Integration Points

- **CLI** (`n3v3-cli`): `n3v3 fmt file/check/dir` commands
- **Parser** (`n3v3-parser`): Formatter consumes parsed AST
- All CLI fmt invocations go through `n3v3-fmt::format()`

## CLI Commands

```bash
n3v3 fmt file path/to/file.n3v3         # Format a file
n3v3 fmt file path/to/file.n3v3 --write # Format and write back
n3v3 fmt check path/to/file.n3v3        # Check if formatted (exit 1 if not)
n3v3 fmt dir .                          # Format all .n3v3 files in directory
```

## Testing

```bash
cargo test -p n3v3-fmt
cargo test --test fmt
```

## Round-trip rules / 往返规则

The printer must emit source that re-parses to the same AST, so these positions
have grammar-driven spellings. Each was a real bug (formatted output did not
parse, or parsed into something else); each is pinned by a test in `tests/fmt.rs`.

打印器必须保证输出可重新解析为同一 AST，因此以下写法由语法决定。每项都曾是真实缺陷
（格式化结果无法解析，或解析成别的东西），现各自有 `tests/fmt.rs` 中的回归测试。

| Construct / 构造 | Printed as / 打印为 | Why / 原因 |
|---|---|---|
| `[x, ..]`, `[x, ..rest]` | `[x, ..]` (comma kept) | `..` always follows `init`; gating the separator on `rest`/`tail` printed the unparseable `[x..]` / 分隔符只取决于 `init`，此前按 `rest`/`tail` 判定会打印出无法解析的 `[x..]` |
| zero-parameter lambda | `fn() { ... }` | `\|\|` lexes as the logical-or token, so an empty parameter list has no `\|...\|` spelling / `\|\|` 是逻辑或 token，空参数列表没有 `\|...\|` 写法 |
| lambda body that is a record or record update | `\|x\| ({ ... })`, `\|x\| ({ base \| field = 1 })` | after lambda parameters `{` starts a **block** (parser `parse_lambda_body`); both record shapes print as `{ ... }` and need parentheses / lambda 参数后的 `{` 是块；两种记录形式都打印为 `{ ... }`，必须加括号 |
| top-level binding whose pattern is not a variable or `_` | `let (a, b) = t`, `let [a, b] = xs` | the keyword-less form only parses for `x = ...` / `_ = ...`; `(a, b) = t` and `[a, b] = xs` are parse errors / 无关键字形式只支持 `x = ...`/`_ = ...`，其余模式去掉 `let` 会无法解析 |
| 1-tuple pattern | `(x,)` (comma kept) | `(x)` is a parenthesized binding and would bind the whole tuple instead of destructuring it / `(x)` 是带括号的绑定，会绑定整个元组而非解构 |

## Gotchas

- `format()` returns `Result`, not `String`. The old `debug_assert!`-based approach was replaced in audit fix M18.
- `would_exceed_width()` is used for basic line wrapping (H11).
- Comments **are** preserved now (`test_format_preserves_comment_text`); the older "drops comments (C6)" note is stale. / 注释现已保留，旧的“丢弃注释（C6）”说法已过时。
- **Shebang lines are dropped**: `n3v3-parser` strips `#!` before parsing (`crates/n3v3-parser/src/lib.rs:39-41`), so the AST never carries it and the formatter cannot re-emit it. `n3v3 fmt file --write` therefore removes the shebang from executable scripts such as `examples/test-runner.n3v3`. Fixing this needs parser/AST plumbing (a `shebang` field on the parsed source), not a printer change. / **shebang 会被丢弃**：解析器在解析前剥离 `#!`，AST 中不存在该信息，打印器无法还原；`n3v3 fmt file --write` 会移除可执行脚本的 shebang。修复需要在解析器/AST 层记录 shebang。
- Top-level `let` and `;` are dropped for variable and `_` patterns (v4.0 canonical form); any other pattern keeps `let` because the keyword-less form would not parse. / 顶层 `let` 与 `;` 对变量与 `_` 模式按设计省略（v4.0 规范形式）；其他模式保留 `let`，否则无关键字形式无法解析。
