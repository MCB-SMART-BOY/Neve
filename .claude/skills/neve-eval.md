# neve-eval: Interpreter

## Architecture

```
   ┌───────────────────┐
   │ Typed HIR Module  │ ← from neve-typeck
   └─────────┬─────────┘
             │
             ▼
   ┌──────────────────────────┐
   │  HIR Evaluator (eval.rs) │
   │                          │
   │  eval_module()           │
   │  eval_expr()             │
   │  eval_call()             │
   │  force() — lazy thunk    │
   │  perform_effect()        │
   └──────────┬───────────────┘
              │
              ▼
       ┌─────────────┐
       │   Value     │
       │   (value.rs)│
       └─────────────┘
```

## Value Model

```rust
pub enum Value {
    // Primitives
    Int(i64),              // 64-bit signed (was BigInt, v1.0)
    Float(f64),            // IEEE 754
    Bool(bool),
    Char(char),            // Unicode scalar value
    Str(String),           // UTF-8
    Unit,

    // Structural
    List(Vec<Value>),                    // Homogeneous list
    Record(Vec<(String, Value)>),         // Named fields
    Tuple(Vec<Value>),                    // Anonymous tuple
    Closure { params: Vec<Name>, body: TypedExpr, env: Rc<Environment> },

    // Runtime objects
    Path(String),                         // ./path, /absolute
    Bytes(Vec<u8>),                       // Binary data
    Command { program: String, args: Vec<String>, opts: CommandOpts },
    Pipeline(Vec<Command>, Vec<Redirect>),
    ProcessResult { code: i32, stdout: String, stderr: String },
    Task(TaskId),

    // Lazy evaluation
    Thunk(Rc<RefCell<ThunkState>>),       // Deferred computation

    // Builtins
    BuiltinFn { name: String, arity: usize },
}

pub enum ThunkState {
    HirUnevaluated { expr: Expr, env: Environment },  // Not yet evaluated
    Evaluating,                                      // Cycle detection
    Evaluated(Value),                                // Memoized result
    Failed(EvalError),                               // Cached failure
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
    fn eval_module(&mut self, module: &TypedModule) -> Result<Value, EvalError> {
        let mut last = Value::Unit;
        for item in &module.items {
            last = self.eval_item(item)?;
        }
        Ok(last)
    }

    fn eval_expr(&mut self, expr: &TypedExpr) -> Result<Value, EvalError> {
        match expr {
            TypedExpr::Lit(lit) => self.eval_literal(lit),
            TypedExpr::Var(def_id) => self.env.lookup(def_id),
            TypedExpr::Lambda { params, body } =>
                Ok(Value::Closure { params: params.clone(), body: *body.clone(), env: self.env.snapshot() }),
            TypedExpr::Call { func, args } => {
                let f = self.eval_expr(func)?;
                let evaluated_args: Vec<_> = args.iter()
                    .map(|a| self.eval_expr(a)).collect::<Result<_,_>>()?;
                self.apply(f, evaluated_args)
            }
            TypedExpr::Let { pattern, value, body } => {
                let v = self.eval_expr(value)?;
                self.env.bind_pattern(pattern, v)?;
                self.eval_expr(body)
            }
            TypedExpr::If { cond, then_branch, else_branch } => {
                match self.eval_expr(cond)? {
                    Value::Bool(true) => self.eval_expr(then_branch),
                    Value::Bool(false) => self.eval_expr(else_branch),
                    other => Err(EvalError::TypeMismatch { expected: "Bool", got: other.type_name() }),
                }
            }
            TypedExpr::Match { scrutinee, arms } => {
                let v = self.eval_expr(scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &v) {
                        self.env.push_scope(bindings);
                        let result = self.eval_expr(&arm.body);
                        self.env.pop_scope();
                        return result;
                    }
                }
                Err(EvalError::MatchExhausted)
            }
            TypedExpr::BinOp(op, lhs, rhs) => self.eval_binop(*op, lhs, rhs),
            TypedExpr::PerformEffect { effect, arg } => self.handle_effect(*effect, arg),
            // ...
        }
    }
}
```

## Effect Handling

```rust
impl Evaluator {
    fn handle_effect(&mut self, effect: EffectId, arg: &TypedExpr) -> Result<Value, EvalError> {
        match effect.as_str() {
            // File I/O
            "io.readFile"   => self.builtin_read_file(arg),
            "io.writeFile"  => self.builtin_write_file(arg),
            // Process
            "io.execCommand"  => self.builtin_exec_command(arg),
            "io.execPipeline" => self.builtin_exec_pipeline(arg),
            // Task
            "io.spawn"     => self.builtin_spawn(arg),
            "io.awaitTask" => self.builtin_await_task(arg),
            "io.cancel"    => self.builtin_cancel(arg),
            // Stream
            "io.streamCollect" => self.builtin_stream_collect(arg),
            "io.streamForEach" => self.builtin_stream_for_each(arg),
            // ...
            _ => Err(EvalError::UnknownEffect(effect)),
        }
    }
}
```

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
| neve-typeck | neve-eval | `TypedModule` |
| neve-eval | neve-cli | `Value` (for REPL/run output) |
| neve-eval | neve-std | `Value` (builtins consume/produce Values) |
| neve-frontend | neve-eval | Orchestrates the parse→typeck→eval pipeline |

## Key Files

| File | What |
|------|------|
| `eval/src/eval.rs` | HIR evaluator — `Evaluator`, `eval_module`, `eval_expr`, `force` |
| `eval/src/value.rs` | Value types + `ThunkState` + `Closure` |
| `eval/src/env.rs` | `Environment` — scoped variable lookup with parent chain |
| `eval/src/pattern.rs` | Pattern analysis — specificity, irrefutability, match hints |
| `eval/src/builtin.rs` | Built-in function registry |
| `eval/src/lib.rs` | Crate root + public API re-exports |

## Error Types

```rust
pub enum EvalError {
    UndefinedVariable(Name),
    TypeMismatch { expected: &'static str, got: String },
    DivisionByZero,
    MatchExhausted,                  // No arm matched (should be caught by typeck)
    PatternBindError(String),
    IOError(std::io::Error),
    UnknownEffect(EffectId),
    Unsupported(String),             // Feature not yet in HIR evaluator
}
```
