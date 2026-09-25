# n3v3-eval: Interpreter

## Architecture

```
   ┌──────────────────────┐
   │ HIR Module           │ ← from n3v3-hir/n3v3-typeck
   └─────────┬────────────┘
             │
             ▼
   ┌──────────────────────────┐
   │ HIR Evaluator (eval.rs)  │
   │                          │
   │  eval_module()           │
   │  eval()                  │
   │  apply() — tail calls    │
   │  force_thunk()           │
   └──────────┬───────────────┘
              │
              ▼
       ┌─────────────┐
       │   Value     │
       │   value.rs  │
       └─────────────┘
```

## Value Model

```rust
pub enum Value {
    // Primitives
    Int(Int),                         // Shared arbitrary-precision integer type
    Float(f64),                       // IEEE 754
    Bool(bool),
    Char(char),                       // Unicode scalar value
    String(Rc<String>),               // UTF-8
    Unit,

    // Runtime objects
    Path(Rc<PathBuf>),
    Bytes(Rc<Vec<u8>>),
    Command(Rc<CommandValue>),
    Pipeline(Rc<PipelineValue>),
    Redirect(Rc<RedirectValue>),
    Task(Rc<TaskValue>),
    Event(Rc<EventValue>),
    Live(Rc<LiveValue>),
    Stream(Rc<StreamValue>),
    ProcessResult(Rc<ProcessResultValue>),

    // Collections
    List(Rc<Vec<Value>>),
    Tuple(Rc<Vec<Value>>),
    Record(Rc<HashMap<String, Value>>),
    Map(Rc<HashMap<String, Value>>),
    Set(Rc<HashSet<String>>),

    // Functions
    Closure { params: Vec<Param>, body: Expr, env: Environment },
    Builtin(BuiltinFn),
    BuiltinFn(&'static str, Rc<dyn Fn(Vec<Value>) -> Result<Value, String>>),

    // Algebraic data types and lazy evaluation
    VariantCtor { name: String, arity: usize },
    VariantCtorWithId { def_id: DefId, name: String, arity: usize },
    VariantWithId { def_id: DefId, name: String, payload: Box<Value> },
    Variant(String, Box<Value>),
    Some(Box<Value>),
    None,
    Ok(Box<Value>),
    Err(Box<Value>),
    Thunk(Thunk),
}

pub enum ThunkState {
    HirUnevaluated { expr: Expr, env: Environment },
    Evaluating,
    Evaluated(Value),
    Failed(EvalError),
}
```

## Lazy Evaluation Flow

```
let x = ~expensive_computation
//
// x is Value::Thunk(HirUnevaluated { expr, env })
//
force(x)
  │
  ├── if Evaluated(v) → return v (memoized)
  │
  └── if HirUnevaluated(expr, env)
        │
        ├── evaluate expr in its captured environment → result
        ├── set thunk to Evaluated(result)
        └── return result
```

Function parameters prefixed with `~` are passed as HIR thunks. The evaluator
forces them at boolean conditions, match guards, list-comprehension conditions,
unary operators, and binary/comparison operations; unused lazy arguments remain
unevaluated. `with_child_env` and function application restore the caller
environment on both success and error paths, including TCO.

## HIR Evaluator Core Loop

```rust
impl Evaluator {
    pub fn eval_module(&mut self, module: &Module) -> Result<Value, EvalError> {
        let mut last = Value::Unit;
        for item in &module.items {
            last = self.eval_item(item)?;
        }
        Ok(last)
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<Value, EvalError> {
        match &expr.kind {
            ExprKind::Literal(lit) => Ok(self.eval_literal(lit)),
            ExprKind::Var(id) => self.env.get(*id).ok_or(EvalError::UnboundVariable),
            ExprKind::Global(def_id) => self
                .global_callable(*def_id)
                .ok_or(EvalError::UnboundVariable),
            ExprKind::Builtin(name) => self
                .builtin_value(name)
                .ok_or_else(|| EvalError::TypeError(format!("unknown builtin: {name}"))),
            ExprKind::Call(func, args) => {
                let func_value = self.eval(func)?;
                let arg_values = self.prepare_call_args(&func_value, args, None)?;
                self.apply(func_value, arg_values)
            }
            ExprKind::If(cond, then_branch, else_branch) => {
                if self.eval_forced(cond)?.is_truthy() {
                    self.eval(then_branch)
                } else {
                    self.eval(else_branch)
                }
            }
            ExprKind::Match(scrutinee, arms) => self.eval_match(scrutinee, arms),
            ExprKind::Lazy(inner) => Ok(Value::Thunk(Thunk::new_hir(
                inner.as_ref().clone(),
                self.env.clone(),
            ))),
            ExprKind::Error(message) => Err(EvalError::TypeError(message.clone())),
            // Record, list, tuple, field, binary, unary, block, and other forms
            // are handled by corresponding branches in eval.rs.
        }
    }
}
```

## Builtin and Effect Dispatch

Effectful calls are represented as `ExprKind::Builtin` values. The evaluator
resolves the builtin name through `builtin_value()` and applies the resulting
callable with the normal call path; there is no separate `PerformEffect` node.

## Memory & Performance

- **Thunks**: Memoized on first force; a failed body caches the error in
  `ThunkState::Failed` so a later `force` reports the same failure without
  re-running side effects. `isEvaluated` stays false for failed thunks.
- **Closures**: Capture `Rc<Environment>` at creation time
- **Value sharing**: `Rc` for structural types (List, Record)
- **Tail-call optimization**: A call in tail position returns `TcoResult::TailCall`
  in `eval_with_tco` and the loop in `apply` iterates instead of recursing. This
  covers direct calls and method calls (`receiver.method(args)`), so tail
  recursion through trait/impl methods does not consume native stack.
- **Deferred actions**: `io.defer` registers into the *current frame*; every frame
  runs its defers when it exits (success or failure) and restores the caller's
  defer scope afterwards. Frames include closures, zero-parameter value bindings,
  the module body, and thunk bodies. A tail call hands its frame to the callee, so
  the caller's defers stay pending until the chain finishes: frames run innermost
  first, and each frame's own actions run last-registered-first. A failing body
  error wins over a cleanup error, and one failing action does not cancel the
  remaining actions of its frame (the first error is reported).
- **Stack budget**: A depth counter cannot trip before native stack exhaustion
  (measured overflow near 1_000 levels), so `apply` also compares the current
  frame address against the evaluator's startup anchor and returns
  `EvalError::TypeError` once the thread's available stack budget
  (`available_stack_bytes()`, 512 KiB reserve, 512 KiB fallback) is spent.
  `MAX_RECURSION_DEPTH` (10_000) remains the hard depth cap for non-tail calls.
- **Fast paths**: Built-in arithmetic avoids thunk overhead

## Integration Points

| From | To | Data |
|------|----|------|
| n3v3-typeck | n3v3-eval | `Module` with typed HIR metadata |
| n3v3-eval | n3v3-cli | `Value` (for REPL/run output) |
| n3v3-eval | n3v3-std | `Value` (builtins consume/produce Values) |
| n3v3-frontend | n3v3-eval | Orchestrates the parse→typeck→eval pipeline |

## Key Files

| File | What |
|------|------|
| `crates/n3v3-eval/src/eval.rs` | HIR evaluator — `Evaluator`, `eval_module`, `eval`, `apply`, `force_thunk` |
| `crates/n3v3-eval/src/value.rs` | Value types + `ThunkState` + `Closure` |
| `crates/n3v3-eval/src/env.rs` | `Environment` — scoped variable lookup with parent chain |
| `crates/n3v3-eval/src/pattern.rs` | Pattern analysis — specificity, irrefutability, match hints |
| `crates/n3v3-eval/src/builtin.rs` | Built-in function registry |
| `crates/n3v3-eval/src/lib.rs` | Crate root + public API re-exports |

## Error Types

```rust
pub enum EvalError {
    UnboundVariable,
    TypeError(String),
    DivisionByZero,
    AssertionFailed(String),
    PatternMatchFailed,
    NotAFunction,
    WrongArity,
    ParseDiagnostics { /* imported-module diagnostics */ },
}
```
