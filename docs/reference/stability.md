# Neve Standard Library Stability Tiers

This document defines the stability guarantees for the Neve standard library (stdlib). Stability tiers are scoped to the v3.x major version. Breaking changes to stable APIs will only occur with a major version bump.

## Tier 1: Stable ✅

**Guarantee**: These APIs are guaranteed not to break within v3.x. They have been extensively exercised in real-world usage and their semantics are well-understood.

### Core I/O

| API | Signature | Description |
|-----|-----------|-------------|
| `print` | `(value: a) -> Unit` | Print value to stdout without newline |
| `println` | `(value: a) -> Unit` | Print value to stdout with newline |
| `io.read` | `() -> String` | Read a line from stdin |
| `io.write` | `(msg: String) -> Unit` | Write string to stdout |
| `io.readFile` | `(path: String) -> String` | Read entire file contents |
| `io.writeFile` | `(path: String, content: String) -> Unit` | Write string to file |
| `io.execCommand` | `(cmd: Command) -> Process` | Execute a command synchronously |
| `io.execPipeline` | `(pipeline: Pipeline) -> List Process` | Execute a pipeline synchronously |
| `io.command` | `(name: String, args: List String) -> Command` | Construct a Command value |
| `io.pipeline` | `(cmds: List Command) -> Pipeline` | Construct a Pipeline value |
| `io.args` | `() -> List String` | Get script arguments |
| `io.getEnv` | `(name: String) -> Option String` | Get environment variable |
| `io.glob` | `(pattern: String) -> List String` | Glob pattern matching |

### Type Conversion

| API | Signature | Description |
|-----|-----------|-------------|
| `toString` | `(value: a) -> String` | Convert any value to string |
| `toInt` | `(value: String) -> Option Int` | Parse string to int |
| `toFloat` | `(value: String) -> Option Float` | Parse string to float |

### List Operations

| API | Signature | Description |
|-----|-----------|-------------|
| `len` | `(list: List a) -> Int` | Length of list or string |
| `head` | `(list: List a) -> Option a` | First element |
| `tail` | `(list: List a) -> Option (List a)` | All but first element |
| `last` | `(list: List a) -> Option a` | Last element |
| `map` | `(f: a -> b, list: List a) -> List b` | Transform each element |
| `filter` | `(f: a -> Bool, list: List a) -> List a` | Keep elements matching predicate |
| `foldl` | `(f: b -> a -> b, init: b, list: List a) -> b` | Left fold |
| `foldr` | `(f: a -> b -> b, init: b, list: List a) -> b` | Right fold |

### Type Introspection

| API | Signature | Description |
|-----|-----------|-------------|
| `typeOf` | `(value: a) -> String` | Get runtime type name |
| `isInt` | `(value: a) -> Bool` | Check if value is Int |
| `isFloat` | `(value: a) -> Bool` | Check if value is Float |
| `isBool` | `(value: a) -> Bool` | Check if value is Bool |
| `isString` | `(value: a) -> Bool` | Check if value is String |

### Path Manipulation

| API | Signature | Description |
|-----|-----------|-------------|
| `path.fromString` | `(s: String) -> Path` | Parse string to Path |
| `path.joinPath` | `(a: Path, b: Path) -> Path` | Join two paths |

### TTY / Terminal

| API | Signature | Description |
|-----|-----------|-------------|
| `io.isTTY` | `(fd: Int) -> Bool` | Check if fd is a terminal |
| `io.terminalSize` | `() -> Option<{rows: Int, cols: Int}>` | Get terminal dimensions |
| `io.setRawMode` | `(fd: Int, enable: Bool) -> Unit` | Set terminal raw mode |
| `io.resetTerminal` | `(fd: Int) -> Unit` | Reset terminal to normal mode |
| `io.readKey` | `(fd: Int) -> Int` | Read a single byte from fd |

### String Operations

| API | Signature | Description |
|-----|-----------|-------------|
| `string.concat` | `(a: String, b: String) -> String` | Concatenate strings |
| `string.contains` | `(s: String, substr: String) -> Bool` | Check substring membership |
| `string.split` | `(s: String, delim: String) -> List String` | Split string by delimiter |

### List Module

| API | Signature | Description |
|-----|-----------|-------------|
| `list.map` | `(f: a -> b, list: List a) -> List b` | Map over list |
| `list.filter` | `(f: a -> Bool, list: List a) -> List a` | Filter list |
| `list.foldl` | `(f: b -> a -> b, init: b, list: List a) -> b` | Left fold |

### Basic Types

All these types are stable:

- `Int` — 64-bit signed integer
- `Float` — 64-bit IEEE 754 float
- `Bool` — Boolean (`true` / `false`)
- `String` — UTF-8 string
- `Char` — Unicode scalar value
- `List a` — Homogeneous list
- `Record` — Named field records
- `Unit` — Unit type (`()`)

### Process Result Accessors

| API | Signature | Description |
|-----|-----------|-------------|
| `io.processSuccess` | `(p: Process) -> Bool` | Whether process exited with code 0 |
| `io.processStdout` | `(p: Process) -> String` | Process stdout |
| `io.processStderr` | `(p: Process) -> String` | Process stderr |
| `io.processCode` | `(p: Process) -> Int` | Process exit code |

---

## Tier 2: Stable but Evolving ⚠️

**Guarantee**: These APIs are stable in their current form but may gain new optional parameters in minor releases. Existing call sites will not break.

### Stream<T> API (14 APIs)

| API | Signature | Description |
|-----|-----------|-------------|
| `io.streamList` | `(list: List a) -> Stream a` | Create stream from list |
| `io.streamLines` | `(path: String) -> Stream String` | Stream lines from file |
| `io.streamCommand` | `(cmd: Command) -> Stream String` | Stream stdout of command |
| `io.streamBytes` | `(path: String) -> Stream Bytes` | Stream raw bytes |
| `io.streamCollect` | `(s: Stream a) -> List a` | Collect stream into list |
| `io.streamMap` | `(s: Stream a, f: a -> b) -> Stream b` | Map over stream |
| `io.streamFilter` | `(s: Stream a, f: a -> Bool) -> Stream a` | Filter stream |
| `io.streamTake` | `(s: Stream a, n: Int) -> Stream a` | Take first n elements |
| `io.streamDrop` | `(s: Stream a, n: Int) -> Stream a` | Drop first n elements |
| `io.streamPipe` | `(s: Stream a, cmd: Command) -> Stream String` | Pipe stream into command |
| `io.streamWrite` | `(s: Stream String, path: String) -> Unit` | Write stream to file |
| `io.streamForEach` | `(s: Stream a, f: a -> Unit) -> Unit` | Apply closure to each element |
| `io.streamFold` | `(s: Stream a, init: b, f: b -> a -> b) -> b` | Left fold over stream |
| `io.streamWithTimeout` | `(s: Stream a, ms: Int) -> Stream (Option a)` | Stream with timeout per element |

### Task Management (7 APIs)

| API | Signature | Description |
|-----|-----------|-------------|
| `io.spawn` | `(task: Task a) -> TaskHandle a` | Spawn a task |
| `io.poll` | `(handle: TaskHandle a) -> Option a` | Poll task for result |
| `io.cancel` | `(handle: TaskHandle a) -> Bool` | Cancel a running task |
| `io.awaitTask` | `(handle: TaskHandle a) -> a` | Block until task completes |
| `io.awaitTasks` | `(handles: List (TaskHandle a)) -> List a` | Await multiple tasks |
| `io.awaitAny` | `(handles: List (TaskHandle a)) -> a` | Await first completing task |
| `io.awaitTaskWithTimeout` | `(handle: TaskHandle a, ms: Int) -> Option a` | Await with timeout |

### Registry CLI (Phase 5)

| CLI Command | Description |
|-------------|-------------|
| `neve registry-update` | Update local registry index |
| `neve registry-serve` | Start local registry server |
| `neve registry-publish` | Publish package to registry |

### File I/O

| API | Signature | Description |
|-----|-----------|-------------|
| `io.readFileLines` | `(path: String) -> List String` | Read file as lines |
| `io.atomicWrite` | `(path: String, content: String) -> Unit` | Atomic file write |
| `io.tempDir` | `() -> String` | Create temporary directory |
| `io.createDirAll` | `(path: String) -> Unit` | Create directory tree |
| `io.removeDirAll` | `(path: String) -> Unit` | Remove directory tree |

### Path Operations

| API | Signature | Description |
|-----|-----------|-------------|
| `path.parentPath` | `(p: Path) -> Option Path` | Parent directory |
| `path.filenamePath` | `(p: Path) -> Option String` | Filename component |
| `path.extensionPath` | `(p: Path) -> Option String` | File extension |

### Signal Handling

| API | Signature | Description |
|-----|-----------|-------------|
| `io.onSignal` | `(signal: String, handler: () -> Unit) -> Unit` | Register signal handler |

### Terminal Introspection

| API | Signature | Description |
|-----|-----------|-------------|
| `io.isTTY` | `() -> Bool` | Check if stdin is a TTY |
| `io.terminalSize` | `() -> {cols: Int, rows: Int}` | Terminal dimensions |

### Bytes Type

| API | Signature | Description |
|-----|-----------|-------------|
| `bytes.fromString` | `(s: String) -> Bytes` | Convert string to bytes |
| `bytes.fromList` | `(list: List Int) -> Bytes` | Convert byte list to Bytes |

---

## Tier 3: Experimental 🔬

**Guarantee**: These APIs may change in minor releases. They represent emerging capabilities that need real-world validation before stabilization. Tier 3 APIs will be promoted to Tier 2 after **2 minor releases** of real-world usage without API changes.

### TTY Raw Mode (2 APIs)

| API | Signature | Description |
|-----|-----------|-------------|
| `io.setRawMode` | `() -> Unit` | Enable raw terminal mode |
| `io.resetTerminal` | `() -> Unit` | Restore terminal settings |

### Job Control (2 APIs)

| API | Signature | Description |
|-----|-----------|-------------|
| `io.jobs` | `() -> List Job` | List background jobs |
| `io.waitAnyJob` | `() -> Job` | Wait for any background job |

### Effect Control Flow

| API | Signature | Description |
|-----|-----------|-------------|
| `io.retry` | `(action: () -> a, maxRetries: Int) -> a` | Retry action on failure |
| `io.ensure` | `(action: () -> a, cleanup: () -> Unit) -> a` | Ensure cleanup runs |
| `io.every` | `(ms: Int, action: () -> Unit) -> Timer` | Periodic execution |
| `io.watchFile` | `(path: String, handler: () -> Unit) -> Watcher` | Watch file for changes |

### Reactive System

| API | Signature | Description |
|-----|-----------|-------------|
| `io.reactive` | `(initial: a, source: Stream a) -> Reactive a` | Create reactive value |
| `io.liveNext` | `(r: Reactive a) -> a` | Get next reactive value |

### Fetch

| API | Signature | Description |
|-----|-----------|-------------|
| `fetch.url` | `(url: String) -> String` | Fetch URL content |
| `fetch.urlWithHash` | `(url: String, hash: String) -> String` | Fetch with integrity check |
| `fetch.git` | `(url: String, rev: String) -> String` | Fetch git repository |

---

## Promotion Process

Tier 3 → Tier 2 promotion requires:

1. **2 minor releases** of real-world usage without API changes.
2. **No open design issues** against the API surface.
3. **Documentation coverage** in the API reference.
4. **Test coverage** in the end-to-end test suite.

Tier 2 → Tier 1 promotion requires:

1. **4 minor releases** of Tier 2 stability.
2. **Widespread adoption** across the Neve ecosystem.
3. **Formal specification** of behavior (where applicable).

## Breaking Change Policy

- **Tier 1**: Breaking changes only with a major version bump (v4.0).
- **Tier 2**: Breaking changes require a deprecation cycle of at least 1 minor release.
- **Tier 3**: Breaking changes may happen in any minor release without prior deprecation.

When a Tier 2 API needs a breaking change, the old API must:
1. Emit a **deprecation warning** at compile time.
2. Be documented as deprecated in the API reference.
3. Remain functional for at least **1 minor release** before removal.

## Versioning

Neve follows **Semantic Versioning** (SemVer):

- **Major** (v3 → v4): Breaking changes to Tier 1 APIs, language syntax, or runtime behavior.
- **Minor** (v3.1 → v3.2): New features, Tier 2 API additions, Tier 3 changes.
- **Patch** (v3.1.0 → v3.1.1): Bug fixes, documentation updates, performance improvements.
