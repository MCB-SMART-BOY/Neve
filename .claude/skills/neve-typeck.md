# neve-typeck: Type System

## Architecture

```
Resolved HIR Module (from neve-hir)
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│  Phase 1: Type Inference (Hindley-Milner W-algorithm)    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ Generate    │→ │ Unify       │→ │ Substitute      │  │
│  │ type vars   │  │ constraints │  │ inferred types   │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
│  Each expression gets a fresh TypeVar; unification       │
│  solves the system of equations                           │
└─────────────────────────────────────────────────────────┘
       │ Inferred monomorphic types
       ▼
┌─────────────────────────────────────────────────────────┐
│  Phase 2: Trait Resolution                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ Collect     │→ │ Solve       │→ │ Verify impl     │  │
│  │ constraints │  │ constraint  │  │ conformance     │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
│  Trait constraints from fn signatures, impl blocks,       │
│  and trait bounds propagate through type variables        │
└─────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│  Phase 3: Pattern & Flow Analysis                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ Exhaustive  │  │ Optional    │  │ Unreachable     │  │
│  │ check       │  │ flow        │  │ pattern warn    │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────┘
       │ Typed HIR + Diagnostics
       ▼
     neve-eval
```

## Type System

### Kinds

```rust
pub enum Kind {
    Type,          // *     — Int, Bool, String, List<T>, ...
    Row,           // {|_|} — Record field rows
    Effect,        // !     — I/O effects
    Fn(Vec<Kind>, Kind),  // * → * — type constructors
}
```

### Type Universe

```rust
pub enum Type {
    // Primitives
    Int, Float, Bool, Char, String, Unit,
    // Compounds
    List(Box<Type>),
    Record(Row),
    Tuple(Vec<Type>),
    Fn { param: Box<Type>, ret: Box<Type>, effects: EffectSet },
    // Polymorphism
    Var(TypeVarId),                // α — type variable
    Forall(Vec<TypeVarId>, Box<Type>),  // ∀α. α → α
    App(TypeId, Vec<Type>),        // List<Int>, Map<K,V>
    // Named
    Named(TypeId, Vec<Type>),      // MyType<T>
    // Runtime objects
    Path, Bytes, Command, Pipeline, ProcessResult, Task(Box<Type>),
}

pub struct Row {
    pub fields: Vec<(Name, Type)>,     // { x: Int, y: String }
    pub rest: Option<Box<Row>>,        // { x: Int | r } — row extension
}
```

## Hindley-Milner Inference

### Algorithm W core

```rust
impl TypeChecker {
    /// Main inference loop
    fn infer_expr(&mut self, env: &TypeEnv, expr: &HIRExpr) -> Result<(Type, Substitution)> {
        match expr {
            HIRExpr::Var { id, .. } => {
                // Instantiate type scheme with fresh variables
                let scheme = self.env.lookup(*id)?;
                Ok(self.instantiate(scheme))
            }
            HIRExpr::Lambda { params, body, .. } => {
                // ∀α. α → β  — fresh vars for params + body
                let param_types: Vec<_> = params.iter()
                    .map(|_| self.fresh_var()).collect();
                let mut extended_env = env.clone();
                for (p, t) in params.iter().zip(param_types.iter()) {
                    extended_env.insert(p, Type::Var(*t));
                }
                let (body_ty, subst) = self.infer_expr(&extended_env, body)?;
                Ok((Type::Fn { param: Box::new(param_types[0].clone()),
                              ret: Box::new(subst.apply(body_ty)),
                              effects: EffectSet::Pure },
                    subst))
            }
            HIRExpr::Call { func, args, .. } => {
                let (fn_ty, s1) = self.infer_expr(env, func)?;
                let arg_ty = self.fresh_var();
                let ret_ty = self.fresh_var();
                self.unify(fn_ty, Type::Fn {
                    param: Box::new(arg_ty),
                    ret: Box::new(ret_ty.clone()),
                    effects: EffectSet::unknown(),
                })?;
                // Infer each argument against parameter types
                Ok((ret_ty, s1))
            }
            // ...
        }
    }
}
```

### Unification (occurs check + substitution)

```rust
fn unify(&mut self, t1: &Type, t2: &Type) -> Result<Substitution> {
    match (t1, t2) {
        (Type::Var(v1), Type::Var(v2)) if v1 == v2 => Ok(Sub::empty()),
        (Type::Var(v), other) => {
            if self.occurs_in(*v, other) {
                Err(TypeError::OccursCheck(*v, other.clone()))
            } else {
                Ok(Sub::singleton(*v, other.clone()))
            }
        }
        (Type::Fn { param: p1, ret: r1, .. },
         Type::Fn { param: p2, ret: r2, .. }) => {
            let s1 = self.unify(p1, p2)?;
            self.unify(&s1.apply(r1), &s1.apply(r2))
        }
        (Type::List(a), Type::List(b)) => self.unify(a, b),
        (Type::Record(r1), Type::Record(r2)) => self.unify_rows(r1, r2),
        (t1, t2) if t1 == t2 => Ok(Sub::empty()),
        _ => Err(TypeError::UnificationMismatch(t1.clone(), t2.clone())),
    }
}
```

## Trait System

### Method dispatch order (Decision G2)

```
1. Inherent impl  → check if Self type has a direct impl
2. Trait method   → check if Self satisfies a trait bound
3. Callable fallback → lower to x.foo(y) → foo(x, y)
4. UnknownMethod  → emit diagnostic
```

```rust
fn resolve_method(&self, recv_ty: &Type, method: &Name) -> Result<ResolvedMethod> {
    // 1. Inherent impl lookup
    if let Some(impl_id) = self.find_inherent(recv_ty, method) {
        return Ok(ResolvedMethod::Inherent(impl_id));
    }
    // 2. Trait method lookup
    for trait_bound in self.env.trait_bounds_for(recv_ty) {
        if let Some(impl_id) = self.find_trait_method(trait_bound, method) {
            return Ok(ResolvedMethod::Trait(impl_id));
        }
    }
    // 3. Callable fallback
    if self.env.has_callable(method) {
        return Ok(ResolvedMethod::CallableFallback);
    }
    // 4. No method found
    Err(TypeError::UnknownMethod(method.clone(), recv_ty.clone()))
}
```

## Exhaustiveness Checking

All `match` expressions must be exhaustive. The checker covers:

| Scrutinee Type | Exhaustiveness Rule |
|----------------|---------------------|
| `Bool` | Both `true` and `false` covered |
| `Int` / `Float` / `Char` / `String` | Requires wildcard `_` arm |
| User enum | All variants covered |
| `Option<T>` | Both `Some` and `None` covered |
| `Result<T,E>` | Both `Ok` and `Err` covered |
| Record | All declared fields covered |
| `List<T>` | Empty (`[]`) + non-empty (`[h, ..t]`) covered |
| Tuple | Per-position coverage |

## Error Diagnostics

```rust
pub enum TypeError {
    OccursCheck(TypeVarId, Type),              // Infinite type
    UnificationMismatch(Type, Type),           // Type conflict
    CannotUnify(Type, Type),                   // Structural mismatch
    MissingImpl(TraitId, Type),                // No impl found
    FieldNotFound(Name, Type),                 // No such record field
    UnknownMethod(Name, Type),                 // Method not found
    NonExhaustiveMatch(Span),                  // Missing match arms
    UnreachablePattern(Span),                  // Dead code in match
    InvalidOptional(Type),                     // ? on non-optional
    InvalidCoalesce(Type),                     // ?? on non-optional
    InvalidSafeAccess(Type),                   // ?. on non-record/option
}
```

## Integration Points

| From | To | Data |
|------|----|------|
| neve-hir | neve-typeck | `HIRModule` (resolved, pre-type) |
| neve-typeck | neve-eval | `TypedModule` (fully typed, ready for evaluation) |
| neve-typeck | neve-lsp | Type map + diagnostics for hover/completion |
| neve-typeck | neve-cli | `neve check` diagnostics |

## Key Files

| File | What |
|------|------|
| `typeck/src/check/mod.rs` | Main type checker (3401 lines) — `TypeChecker`, `infer_module`, `check` |
| `typeck/src/check/builtin_type.rs` | Built-in type checking (1494 lines) |
| `typeck/src/infer.rs` | Hindley-Milner W-algorithm |
| `typeck/src/unify.rs` | Unification + substitution |
| `typeck/src/constraints.rs` | Trait constraint generation |
| `typeck/src/solve.rs` | Trait constraint solver |
| `typeck/src/types.rs` | Type, Kind, TypeScheme definitions |
| `typeck/src/rows.rs` | Row polymorphism for records |
| `typeck/src/traits.rs` | Trait resolution + method dispatch |
| `typeck/src/effects.rs` | Effect inference + checking |
