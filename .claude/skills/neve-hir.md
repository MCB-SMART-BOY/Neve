# neve-hir: High-Level Intermediate Representation

## Architecture

```
SourceFile (AST) from neve-parser
       │
       ▼
┌─────────────────────────────────────────────────┐
│  Phase 1: Name Resolution (resolve.rs)           │
│  ┌────────────┐  ┌──────────┐  ┌──────────────┐ │
│  │ ScopeStack │→ │ DefTable │→ │ Resolved     │ │
│  │ push/pop   │  │ insert   │  │ identifiers  │ │
│  └────────────┘  └──────────┘  └──────────────┘ │
│  Walks AST items; resolves use statements;       │
│  builds module graph                              │
└─────────────────────────────────────────────────┘
       │ Resolved names + module graph
       ▼
┌─────────────────────────────────────────────────┐
│  Phase 2: Lowering (module_lowering.rs)          │
│  ┌────────────┐  ┌──────────┐  ┌──────────────┐ │
│  │ AST→HIR    │→ │ Pattern  │→ │ Effect       │ │
│  │ transformer│  │ lowering │  │ inference    │ │
│  └────────────┘  └──────────┘  └──────────────┘ │
│  Transforms AST nodes to HIR nodes;              │
│  resolves pattern bindings; marks effects         │
└─────────────────────────────────────────────────┘
       │ Resolved HIR Module
       ▼
┌─────────────────────────────────────────────────┐
│  Phase 3: Module Graph (module_paths.rs)         │
│  ┌────────────┐  ┌──────────┐  ┌──────────────┐ │
│  │ Path       │→ │ ModuleId │→ │ Loaded       │ │
│  │ resolution │  │ mapping  │  │ Module cache │ │
│  └────────────┘  └──────────┘  └──────────────┘ │
│  Handles: self, super, crate, relative, absolute  │
│  Flake-aware: resolves flake inputs as modules    │
└─────────────────────────────────────────────────┘
```

## AST → HIR Transformation

| AST Node | HIR Node | Key Change |
|----------|----------|------------|
| `Expr::Var("x")` | `HIRExpr::Var { id: DefId }` | Name → resolved DefId |
| `Expr::Call(f, args)` | `HIRExpr::Call { func, args }` | Function name resolved |
| `Expr::Lambda(params, body)` | `HIRExpr::Lambda { params, body }` | Param binding resolved |
| `x.foo(y)` | `HIRExpr::MethodCall { .. }` or fallback | Method dispatch resolved |
| `Expr::Match(s, arms)` | `HIRExpr::Match { .. }` | Pattern bindings resolved |
| `Item::Use("std.list")` | `HIRItem::Use { module_id }` | Path → ModuleId |
| `expr?` | `HIRExpr::Try { .. }` | Optional-flow wrapper |
| `lazy expr` | `HIRExpr::Lazy { .. }` | Thunk wrapper |

## Name Resolution Strategy

```
Scope stack (lexically nested):
┌─────────────────────────────────┐
│  Module scope (top-level items) │ ← global definitions
│  ┌───────────────────────────┐  │
│  │  Block scope { ... }      │  │ ← let bindings in blocks
│  │  ┌─────────────────────┐  │  │
│  │  │  Lambda scope |x|.. │  │  │ ← parameter bindings
│  │  └─────────────────────┘  │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

Resolution order for `use`:
1. `self` → current module
2. `super` → parent module
3. `crate` → workspace root
4. `std.*` → standard library (built-in overrides)
5. Relative path → resolved from current file's directory
6. Flake input → resolved from `flake.neve`

## Key Types

```rust
// Resolved HIR module
pub struct HIRModule {
    pub items: Vec<HIRItem>,
    pub defs: DefTable,           // All definitions in scope
    pub module_id: ModuleId,      // This module's identity
    pub imports: Vec<ModuleId>,   // Resolved dependencies
}

// Definition table (shared across the module graph)
pub type DefId = usize;
pub struct DefTable {
    entries: Vec<DefEntry>,
}
pub enum DefEntry {
    Value { name: String, ty: Option<Type>, span: Span },
    Function { name: String, generics: Vec<Generic>, .. },
    Type { name: String, def: TypeDef, .. },
    Module { module_id: ModuleId },
}

// Resolved expression
pub enum HIRExpr {
    Var { id: DefId, span: Span },           // Name → DefId
    Field { recv: Box<HIRExpr>, field: Name, def: DefId },
    Call { func: Box<HIRExpr>, args: Vec<HIRExpr> },
    MethodCall { recv: Box<HIRExpr>, method: Name, args, resolved: MethodId },
    Lambda { params: Vec<(Name, DefId)>, body: Box<HIRExpr> },
    Match { scrutinee: Box<HIRExpr>, arms: Vec<MatchArm> },
    Let { pattern: HIRPattern, value: Box<HIRExpr>, body: Box<HIRExpr> },
    Lazy { expr: Box<HIRExpr> },
}
```

## Integration Points

| From | To | Data |
|------|----|------|
| neve-parser | neve-hir | `SourceFile` (AST) |
| neve-hir | neve-typeck | `HIRModule` (resolved, ready for type inference) |
| neve-hir | neve-frontend | `DefTable` + module graph side tables |

## Key Files

| File | What |
|------|------|
| `hir/src/hir.rs` | HIR node definitions |
| `hir/src/def.rs` | DefTable, DefId, DefEntry |
| `hir/src/resolve.rs` | Name resolution (scope stack, def registration) |
| `hir/src/module_lowering.rs` | AST → HIR transformation |
| `hir/src/module_paths.rs` | Module path resolution (self/super/crate/flake) |

## Testing

- **Lowering fidelity**: Every pattern form must survive lowering without loss
- **Lossy lowering is a bug**: If a construct degrades to wildcard, it must be fixed
- **Module resolution**: Test self/super/crate/relative/absolute/`use as` paths
