# Neve Effect Boundary Design

This document defines the effect model for Neve — the boundary between pure functional code and host-side effects (I/O, process execution, network).

本文档定义 Neve 的副作用模型 — 纯函数代码与宿主机副作用（I/O、进程执行、网络）之间的边界。

## 1. Status

- State: **shipping phase** — effect keyword done, streaming I/O + timeout complete, signal handling, non-blocking tasks, pipe syntax (v3.5.0)
- Scope: effect classification, purity checking, effect type system design, streaming timeout
- Related: G4 (Effect Boundary), PR-011/012 (typed runtime objects), `neve check --pure`

## 2. Design Principles

1. **Core language is pure** — evaluation of Neve expressions has no side effects
2. **Effects are explicit** — effectful operations are only reachable through typed runtime entrypoints
3. **Effects are runtime-mediated** — the evaluator does not directly touch the host; it delegates to builtin functions
4. **Purity is checkable** — `neve check --pure` can statically reject effectful code
5. **Gradual adoption** — users can start pure and add effects incrementally

## 3. Current State

### 3.1 What Exists

| Component | Status | Description |
|-----------|--------|-------------|
| Typed runtime objects | Done | Path, Bytes, Command, Pipeline, Redirect, ProcessResult, Task<T> |
| Task<T> | Done | Wraps Command/Pipeline for deferred execution; blocking await only |
| io.awaitTaskWithTimeout | Done | Timeout + process kill for Command-based tasks |
| Streaming I/O | Done | execCommandStreaming, execPipelineStreaming, readFileLines (line-by-line callback) |
| Streaming timeout | Done | execCommandStreamingWithTimeout, execPipelineStreamingWithTimeout (total deadline + process kill) |
| Streaming safety limits | Done | max lines (100k), max stdin (10MB), max intermediate buffer (50MB) enforced in evaluator |
| Signal handling | Done | io.onSignal registers OS signal handlers (INT/TERM/HUP/USR1/USR2); evaluator polls atomic flags and dispatches callbacks at safe points |
| neve check --pure | Done | HIR walker rejects calls to effectful builtins |
| is_effectful_builtin() | Done | Classifies stdlib builtins as pure or effectful |
| Process inspectors | Done | processSuccess, processStdout, processCode, processStderr (pure) |

### 3.2 What's Missing

| Gap | Priority | Description |
|-----|----------|-------------|
| Effect type system | High | No way to express "this function may have effects" in the type system |
| Effect polymorphism | Medium | Can't write functions generic over purity |
| Non-blocking Task | Medium | Tasks always block; no poll/cancel/background execution |
| Effect inference | Low | Purity is opt-in (--pure flag) rather than inferred |

## 4. Effect Classification

### 4.1 Effect Categories

| Category | Examples | Current Status |
|----------|----------|----------------|
| **Pure** | list.map, math.abs, string.len, option.some | Always allowed |
| **FileSystem** | io.readFile, io.writeFile, io.readDir | Tracked by is_effectful_builtin |
| **Process** | io.execCommand, io.execPipeline | Tracked by is_effectful_builtin |
| **Network** | fetch.url, fetch.git | Tracked by is_effectful_builtin |
| **Environment** | io.getEnv, io.currentDir, io.homeDir | Tracked by is_effectful_builtin |
| **Inspector** | io.processSuccess, io.processStdout | Pure (read-only access to existing result) |

### 4.2 Current Effectful Builtins

All builtins under `io.*` except the four ProcessResult inspectors (`processSuccess`, `processStdout`, `processCode`, `processStderr`) are classified as effectful. All `fetch.*` builtins are effectful. All other stdlib modules (`list`, `map`, `set`, `math`, `option`, `result`, `string`, `path`) are pure.

## 5. Proposed Effect Type System

### 5.1 Phase 1: Effect Annotations (Minimal)

Add an `effect` keyword for function signatures:

```neve
// Pure function (default)
fn add(x: Int, y: Int) -> Int = x + y;

// Effectful function
fn readConfig(path: String) -> String effect { io.readFile(path) };

// Effectful function calling other effectful functions
fn buildProject(name: String) -> ProcessResult effect {
    let config = readConfig("build.neve");
    io.execCommand(io.command("make", [name]))
};
```

Rules:
- Functions without `effect` annotation cannot call effectful builtins or effectful functions
- `effect` is transitive: calling an effectful function makes the caller effectful
- `neve check` enforces this statically (upgrades current `--pure` opt-in to default)
- `neve check --allow-effects` restores current permissive behavior

### 5.2 Phase 2: Effect Polymorphism (Future)

Allow generic code over purity:

```neve
// map is pure regardless of input function purity
fn map<A, B>(f: fn(A) -> B, xs: List<A>) -> List<B> = ...;

// Higher-order functions propagate effects
fn tap<A>(f: fn(A) -> Unit effect, x: A) -> Unit effect = { f(x); () };
```

### 5.3 Phase 3: Effect Handlers (Future)

Allow catching and handling effects:

```neve
// Timeout as an effect handler
let result = io.awaitTaskWithTimeout(task, 5000) handle {
    Timeout -> None
};
```

## 6. Streaming I/O Design

### 6.1 Problem

Current `io.execCommand` buffers all output. For long-running commands or large outputs, this is impractical.

### 6.2 Proposed: Callback-based Streaming

```neve
// Line-by-line processing via callback
io.execCommandLines(
    io.command("tail", ["-f", "/var/log/system.log"]),
    fn(line: String) -> Unit effect {
        io.writeFile("/tmp/filtered.log", line);
    }
);
```

### 6.3 Proposed: Iterator-based Streaming (Future)

When Neve has lazy sequences:

```neve
// Lazy line iterator
let lines: Stream<String> = io.execCommandStream("journalctl", ["-f"]);
let filtered = stream.filter(fn(line) line.contains("error"), lines);
```

## 7. Implementation Plan

### Step 1: Effect Annotation Parsing (est. 1-2 sessions)
- Add `effect` keyword to lexer/parser
- Add `effectful: bool` to HIR FnDef
- Update lowering to propagate effect annotations

### Step 2: Effect Checking in Typeck (est. 1-2 sessions)
- Add effect tracking to TypeChecker
- When checking a function body, verify all calls are to pure functions (unless annotated `effect`)
- Report errors for effectful calls in pure contexts
- Replace `--pure` opt-in with default-on checking; add `--allow-effects` to restore old behavior

### Step 3: Streaming Output (est. 1-2 sessions)
- Add `io.execCommandLines` with callback
- Implement line-by-line process output
- Add full-pipeline parity tests

### Step 4: Pipeline Timeout (est. 1 session)
- Extend `awaitTaskWithTimeout` to spawn pipeline processes with timeouts
- Kill all pipeline processes on timeout

## 8. Migration Path

1. ~~Current~~ → **Done**: `neve check` defaults to rejecting effects; `--allow-effects` for migration
2. **Phase 1** ✅: `effect` keyword available; `neve check` defaults to rejecting effects
3. **Phase 2**: Effect polymorphism for higher-order functions
4. **Phase 3**: Effect handlers and streaming

## 9. Acceptance Criteria

- [ ] `effect` keyword parses and lowers correctly
- [ ] Pure functions calling effectful builtins produce clear errors
- [ ] Effectful functions can call other effectful functions
- [ ] `neve check` enforces purity by default
- [ ] Streaming process output works end-to-end
- [ ] Pipeline timeout works with process kill
- [ ] All existing tests pass with new effect checking
- [ ] Feature matrix updated
