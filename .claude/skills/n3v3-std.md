# n3v3-std: Standard Library

## Module Architecture

```
crates/n3v3-std/src/lib.rs
    │
    ├── io/mod.rs        ← I/O, filesystem, process, Task, Stream<T>, TTY
    │   ├── File operations (read/write/append/atomic/copy/move)
    │   ├── Directory operations (create/remove/walk/glob/temp)
    │   ├── Process execution (Command, Pipeline, ProcessResult, Redirect)
    │   ├── Task management (spawn/poll/cancel/awaitAny/awaitWithTimeout)
    │   ├── Stream<T> (13 Stream<T> APIs: construct → transform → consume)
    │   ├── Environment (getEnv/setEnv/env/cwd)
    │   ├── TTY (isTTY/terminalSize/setRawMode/resetTerminal/readKey)
    │   └── Signals (onSignal)
    │
    ├── list.rs         ← List operations
    ├── map.rs          ← Map operations
    ├── set.rs          ← Set operations
    ├── string.rs       ← String operations
    ├── option.rs       ← Option operations
    ├── result.rs       ← Result operations
    ├── path.rs         ← Path operations
    ├── math.rs         ← Math operations
    ├── bytes.rs        ← Byte operations
    └── fetch.rs        ← URL/Git fetching
```

## Effect Classification

Every stdlib function is classified by the shared registry:

```rust
// crates/n3v3-common/src/lib.rs
pub fn is_effectful_builtin(name: &str) -> bool {
    // Single-segment aliases and module-qualified names are classified here.
    // Pure constructors/inspectors are excluded from the effectful set.
    /* matches!(name, ...) */
}

// crates/n3v3-std/src/lib.rs delegates to the shared registry:
pub fn is_effectful_builtin(name: &str) -> bool {
    n3v3_common::is_effectful_builtin(name)
}
```

`n3v3-common::is_effectful_builtin` is the single source of truth shared by
`n3v3-typeck` and `n3v3-std`.

| Function | Input | Output | Effect |
|----------|-------|--------|--------|
| `io.readFile` | `String` | `String` | IO |
| `io.readFilePath` | `Path` | `String` | IO |
| `io.readFileBytesPath` | `Path` | `Bytes` | IO |
| `io.writeFile` | `String, String` | `Unit` | IO |
| `io.writeFilePath` | `Path, String` | `Unit` | IO |
| `io.writeFileBytesPath` | `Path, Bytes` | `Unit` | IO |
| `io.atomicWrite` | `String, String` | `Unit` | IO |
| `io.copy` | `String, String` | `Unit` | IO |
| `io.move` | `String, String` | `Unit` | IO |

## I/O — Process Execution

```
Command construction:
  io.command("echo", ["hello"])
       │
       ▼
  Command { program: "echo", args: ["hello"], cwd: None, env: None, stdin: None }
       │
       ├── io.execCommand(cmd) → ProcessResult { code, stdout, stderr }
       ├── io.taskCommand(cmd) → Task<ProcessResult>
       └── io.execCommandStreaming(cmd) → Stream<String>
## I/O — Task Lifecycle

```
io.taskCommand(cmd)
       │
       ▼
  Task<ProcessResult>
       │
       ├── io.awaitTask(task)         → ProcessResult (blocking)
       ├── io.awaitTaskWithTimeout(ms) → Option<ProcessResult> (timeout)
       ├── io.spawn(task)             → spawn_id: Int
       │     ├── io.poll(spawn_id)    → Option<ProcessResult> (non-blocking)
       │     ├── io.cancel(spawn_id)  → Unit
       │     └── io.awaitAny(tasks)   → ProcessResult (first to complete)
       └── io.awaitTasks(tasks)       → List<ProcessResult> (all complete)
## Stream<T> — 13 Stream<T> APIs

```
Construction          Transformation       Consumption           Combinators
───────────          ──────────────       ───────────           ───────────
streamList(list)     streamMap(s, f)      streamCollect(s)      streamWithTimeout(s, ms)
streamLines(path)    streamFilter(s, p)   streamPipe(s, cmd)
streamCommand(cmd)   streamTake(s, n)     streamForEach(s, f)
streamBytes(path)    streamDrop(s, n)     streamFold(s, init, f)
```
