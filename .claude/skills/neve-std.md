# neve-std: Standard Library

## Module Architecture

```
neve-std/src/lib.rs
    │
    ├── io.rs           ← I/O, filesystem, process, Task, Stream<T>, TTY
    │   ├── File operations (read/write/append/atomic/copy/move)
    │   ├── Directory operations (create/remove/walk/glob/temp)
    │   ├── Process execution (Command, Pipeline, ProcessResult, Redirect)
    │   ├── Task management (spawn/poll/cancel/awaitAny/awaitWithTimeout)
    │   ├── Stream<T> (14 APIs: construct → transform → consume)
    │   ├── Environment (getEnv/setEnv/env/cwd)
    │   ├── TTY (isTTY/terminalSize/setRawMode/resetTerminal/readKey)
    │   └── Signals (onSignal)
    │
    ├── list.rs         ← 26 operations (map/filter/fold/head/tail/...)
    ├── string.rs       ← 15 operations (split/join/trim/upper/lower/...)
    ├── option.rs       ← 7 operations (map/flatMap/getOrElse/isSome/...)
    ├── result.rs       ← 7 operations (map/flatMap/isOk/unwrap/...)
    ├── path.rs         ← 11 operations (join/parent/filename/extension/...)
    ├── math.rs         ← 18 operations (floor/ceil/sqrt/log/sin/cos/pi/e/...)
    ├── bytes.rs        ← 7 operations (len/concat/fromString/fromList/...)
    ├── fmt.rs          ← Formatting helpers
    └── net.rs          ← Networking (scaffolded)
```

## Effect Classification

Every stdlib function is classified:

```rust
// In neve-std/src/lib.rs
pub fn is_effectful_builtin(name: &str) -> bool {
    matches!(name,
        // File I/O (Effect::IO)
        "io.readFile" | "io.writeFile" | "io.appendFile" |
        "io.readFileBytes" | "io.writeFileBytes" | "io.atomicWrite" |
        "io.copy" | "io.move" |
        // Directory (Effect::IO)
        "io.createDirAll" | "io.removeDirAll" | "io.walk" | "io.tempDir" |
        "io.glob" |
        // Process (Effect::IO)
        "io.execCommand" | "io.execPipeline" |
        "io.execCommandStreaming" | "io.execPipelineStreaming" |
        // Environment (Effect::IO)
        "io.getEnv" | "io.setEnv" | "io.env" | "io.cwd" | "io.setCwd" |
        // Task (Effect::Async)
        "io.taskCommand" | "io.taskPipeline" |
        "io.spawn" | "io.cancel" | "io.poll" |
        "io.awaitTask" | "io.awaitTasks" | "io.awaitAny" | "io.awaitTaskWithTimeout" |
        // TTY (Effect::IO)
        "io.isTTY" | "io.terminalSize" | "io.setRawMode" |
        "io.resetTerminal" | "io.readKey" |
        // Signals (Effect::IO)
        "io.onSignal" |
        // Streaming (Effect::IO)
        "io.readFileLines" |
        "io.execCommandStreamingWithTimeout" | "io.execPipelineStreamingWithTimeout" |
        // Fetch (Effect::Network)
        "fetch.url" | "fetch.git"
    )
}
```

## Module Override Mechanism

The stdlib is injected into the AST evaluator via `std_module_overrides()`:

```rust
pub fn std_module_overrides() -> HashMap<Vec<String>, Rc<AstEnv>> {
    // Each stdlib module is pre-built as an AstEnv
    // Key: module path segments (e.g., ["std", "list"])
    // Value: pre-populated environment with function bindings
}
```

## I/O — File Operations

| Function | Input | Output | Effect |
|----------|-------|--------|--------|
| `io.readFile` | `String` | `String` | IO |
| `io.readFilePath` | `Path` | `String` | IO |
| `io.readFileBytes` | `String` | `Bytes` | IO |
| `io.writeFile` | `String, String` | `Unit` | IO |
| `io.writeFilePath` | `Path, String` | `Unit` | IO |
| `io.atomicWrite` | `Path, String` | `Unit` | IO |
| `io.copy` | `Path, Path` | `Unit` | IO |
| `io.move` | `Path, Path` | `Unit` | IO |

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
```

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
```

## Stream<T> — 14 APIs

```
Construction          Transformation       Consumption           Combinators
───────────          ──────────────       ───────────           ───────────
streamList(list)     streamMap(s, f)      streamCollect(s)      streamWithTimeout(s, ms)
streamLines(path)    streamFilter(s, p)   streamPipe(s, cmd)
streamCommand(cmd)   streamTake(s, n)     streamForEach(s, f)
streamBytes(path)    streamDrop(s, n)     streamFold(s, init, f)
                     streamWrite(s, path)
```

## AST Compatibility Bridge

`neve-std` provides `std_module_overrides()` for the **deprecated** AST evaluator path. When the AST compat layer is removed in v4.0, this interface will be removed or replaced with an HIR-native module loading mechanism.

## Integration Points

| From | To | Data |
|------|----|------|
| neve-std | neve-eval | `std_module_overrides()` → maps module paths to pre-built `AstEnv`s |
| neve-std | neve-typeck | `is_effectful_builtin()` → effect classification for type checker |
| neve-std | all consumers | `Value`, `Command`, `ProcessResult` type definitions |

## Key Files

| File | What |
|------|------|
| `std/src/lib.rs` | Module declarations + `std_module_overrides()` + `is_effectful_builtin()` |
| `std/src/io/mod.rs` | I/O builtins — files, processes, tasks, streams, TTY, signals |
| `std/src/list.rs` | List operations |
| `std/src/string.rs` | String operations |
| `std/src/path.rs` | Path manipulation |
| `std/src/option.rs` | Option type + operations |
| `std/src/result.rs` | Result type + operations |
| `std/src/math.rs` | Math operations + constants |
| `std/src/bytes.rs` | Bytes type + operations |
