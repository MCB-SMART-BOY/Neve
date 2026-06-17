---
name: run-neve
description: Build, run, smoke-test, and drive the Neve language toolchain (repl, eval, check, fmt, lsp). Use this when asked to run Neve, test Neve CLI behavior, or verify a change works.
---

# Run Neve

Neve is a pure functional language for system configuration, built as a
Rust Cargo workspace. The CLI binary (`neve`) is at `./target/debug/neve`
(debug) or `./target/release/neve` (release). Paths in this document are
relative to the repo root.

## Prerequisites

```bash
sudo apt-get install -y build-essential pkg-config libssl-dev
```

Rust toolchain is assumed present (`rustc`, `cargo`). If missing:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

## Build

```bash
cargo build -p neve          # debug — fast compile, for dev iteration
cargo build -p neve --release  # release — for performance benchmarks
```

## Run (agent path) — smoke driver

The driver exercises every CLI command programmatically and exits 0
if all checks pass:

```bash
.claude/skills/run-neve/driver.sh          # debug binary
.claude/skills/run-neve/driver.sh release  # release binary
```

It covers: `eval`, `run`, `check`, `repl` (piped input), `fmt file`,
`fmt check`, `lsp --check`, v3.0 syntax validation, and error diagnostics.

### Individual commands

```bash
BIN=./target/debug/neve

# Evaluate an expression
echo '1 + 2 * 3' | $BIN repl          # → 7 (interactive)
$BIN eval "1 + 2 * 3"                 # → 7 (non-interactive)

# Run a .neve file
$BIN run examples/hello.neve

# Type-check a file (effects on by default)
$BIN check path/to/file.neve
$BIN check --allow-effects path/to/file.neve

# Format a file
$BIN fmt file path/to/file.neve        # prints to stdout
$BIN fmt check path/to/file.neve       # exit 0 if already formatted

# LSP health check
$BIN lsp --check                       # 7-point diagnostic

# Start LSP server (stdio transport)
$BIN lsp

# REPL with piped input (non-interactive)
printf '1 + 2\n:quit\n' | $BIN repl
```

### REPL (interactive, under tmux)

For tests that need a real interactive REPL session (screenshots, tab
completion), run under tmux:

```bash
tmux new-session -d -s neve-repl './target/debug/neve repl'
tmux send-keys -t neve-repl '1 + 2' Enter
sleep 0.3
tmux capture-pane -t neve-repl -p > /tmp/repl-screenshot.txt
tmux send-keys -t neve-repl ':quit' Enter
tmux kill-session -t neve-repl
```

## Run (human path)

```bash
cargo run -p neve -- repl     # interactive REPL
cargo run -p neve -- run file.neve
```

## Test

```bash
cargo test --workspace                     # unit + integration
cargo test --test end_to_end -- --nocapture  # 541 E2E tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Gotchas

- **`neve fmt file` outputs to stdout, does not modify the file.** Redirect
  to a new file and move it back if you want in-place formatting.
- **`neve fmt check` returns non-zero exit on unformatted files.** This is
  correct — it means "would reformat."
- **`neve check` rejects effectful calls by default.** Use
  `--allow-effects` to bypass purity checking.
- **REPL stdin piping works for simple input** but multi-line programs
  with indentation may need the tmux approach above.
- **LSP health check requires filesystem state** — git repo, Helix
  grammar/queries. It may report fewer checks outside a full checkout.
- **The debug binary prints version 3.19.0** — version bump is done in
  a post-release commit. The actual feature set is 3.19.0.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `cargo build` fails with "linker not found" | `sudo apt-get install build-essential` |
| `cargo build` fails with "ssl" errors | `sudo apt-get install libssl-dev` |
| `neve lsp --check` reports missing grammar | Run `neve setup helix` first |
| REPL hangs with piped input | Use `printf` with explicit `:quit\n` terminator |
| `neve fmt check` fails after `neve fmt file` | The file wasn't overwritten — redirect stdout |

## Driver

[driver.sh](driver.sh) — 100-line smoke test covering all 9 CLI paths.
Run it from the repo root. It creates a temp directory, writes .neve
test files, exercises every command, and cleans up.
