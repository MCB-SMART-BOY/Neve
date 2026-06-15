# neve-parser: Lexer & Parser

## Architecture

```
Source Code (.neve)
       │
       ▼
┌────────────────────────────────────────────────────┐
│  neve-lexer (logos derive macro)                    │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────┐ │
│  │ char stream  │→ │ token stream │→ │ [Spanned  │ │
│  │ (peek/adv)   │  │ (Token enum) │  │ <Token>]  │ │
│  └──────────────┘  └──────────────┘  └───────────┘ │
│  Handles: int/float/hex/oct/bin, strings, comments, │
│  chars, identifiers, operators, delimiters           │
└────────────────────────────────────────────────────┘
       │ [Spanned<Token>]
       ▼
┌────────────────────────────────────────────────────┐
│  neve-parser (recursive descent, LL(1))             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────┐  │
│  │ token stream │→ │ Pratt        │→ │ AST      │  │
│  │ (pos cursor) │  │ precedence   │  │ Module   │  │
│  └──────────────┘  └──────────────┘  └──────────┘  │
│  Handles: module→items→exprs→patterns→types        │
└────────────────────────────────────────────────────┘
       │ Module { items: Vec<Item>, span }
       ▼
┌────────────────────────────────────────────────────┐
│  neve-syntax (AST node definitions)                 │
│  Expr, Item, Pattern, Type, Lit, SourceFile, Span   │
└────────────────────────────────────────────────────┘
```

## Syntax v3.0 Transformations

| Old Form | v3.0 Form | Rationale |
|----------|-----------|-----------|
| `struct Foo {}` / `enum Bar {}` | `type Foo = {}` / `type Bar = \| ...` | Unified type declaration |
| `import std.list` | `use std.list` | Shorter, Rust-aligned |
| `fn(x) x + 1` | `|x| x + 1` | Rust-style lambda |
| `#{ x = 1 }` | `{ x = 1 }` | Delimiter-driven container theory |
| `// comment` | `& comment` | `//` freed for path operator |
| `a // b` (merge) | `a & b` | Consistent with `&` comment |
| `let`/`fn`/`;` required | Optional at top level | Less ceremony |

**Backward compatibility**: The lexer still accepts legacy keywords. The parser's `parse_item()` accepts both old and new syntax forms.

## Lexer Design (neve-lexer)

### Token dispatch flow

```
peek_char()
    │
    ├── digit/number → number() → Int | Float
    ├── '"'           → string() → Str
    ├── '\''          → char()   → Char
    ├── '`'           → interpolated() → InterpolatedStr
    ├── letter/_      → ident()  → Ident | Keyword
    ├── '/'           → path or div
    ├── '&'           → & (comment) or && (and)
    ├── '|'           → |, |>, ||
    ├── '{'/'}'/'('...→ Delimiter
    └── ...
```

### Known gap: `\u{...}` escapes
Unicode escapes in char/string literals not yet supported. Tracked as `#[ignore]` test at `tests/parser.rs:1620`.

## Parser Design (neve-parser)

### Expression precedence (Pratt parser)

```
1.  .  ?.  ()  []           ← primary/postfix
2.  ?                        ← postfix error propagation
3.  !  -                     ← prefix unary
4.  ^                        ← power
5.  *  /  %                  ← multiplicative
6.  +  -                     ← additive
7.  ++                       ← concatenation
8.  <  <=  >  >=  ==  !=     ← comparison
9.  &&                       ← logical and
10. ||                       ← logical or
11. ??                       ← null coalescing
12. |>                       ← pipe
13. &                        ← record merge
```

### Recursive descent structure

```
parse_module()
  └── parse_item() × N
        ├── parse_let()      → LetBinding
        ├── parse_fn()       → FnDef
        ├── parse_type()     → TypeDef (struct/enum unified)
        ├── parse_trait()    → TraitDef
        ├── parse_impl()     → ImplBlock
        ├── parse_use()      → UseStatement
        └── parse_expr()     → Expr (top-level expression tail)

parse_expr(0)  ← Pratt entry at minimum binding power
  └── parse_atom()
        ├── literal (int/float/string/char/bool/unit)
        ├── path literal (./ ../ /)
        ├── identifier or call
        ├── |params| body   ← lambda (v3.0)
        ├── { fields }      ← record (v3.0)
        ├── [ items ]       ← list
        ├── ( expr )        ← grouping or tuple
        ├── if/then/else
        ├── match { arms }
        └── lazy expr
```

## AST Types (neve-syntax)

```rust
pub struct Module {
    pub items: Vec<Item>,
    pub span: Span,
}
pub struct SourceFile {
    pub module: Module,
    pub diagnostics: Vec<Diagnostic>,
}
pub enum Item {
    Let(LetBinding),   // x = expr
    Fn(FnDef),         // f(x) = expr
    Type(TypeDef),     // type Foo = { ... } | ...
    Trait(TraitDef),   // trait Show { ... }
    Impl(ImplBlock),   // impl Show for Foo { ... }
    Use(UseStatement), // use std.list
}
pub enum ExprKind {
    Lit(Literal),              Call { .. },
    Var(Name),                 Lambda { .. },
    Record(Vec<Field>),        Match { .. },
    List(Vec<Expr>),           BinOp { .. },
    Block(Vec<Stmt>, Box<Expr>),
    TupleIndex { expr, idx },
    Optional { .. },           Pipe { .. },
}
```

## Integration Points

| From | To | Data |
|------|----|------|
| neve-lexer | neve-parser | `Vec<Spanned<Token>>` |
| neve-parser | neve-hir | `SourceFile` |
| neve-parser | neve-fmt | `SourceFile` (round-trip) |
| neve-parser | neve-lsp | `SourceFile` (for analysis) |

## Key Files

| File | What |
|------|------|
| `neve-lexer/src/lexer.rs` | Lexer — `peek_char()`, `advance()`, `number()`, `ident()`, `skip_block_comment()` |
| `neve-lexer/src/token.rs` | Token enum — Int, Float, Str, Ident, Keywords, Delimiters, Operators |
| `neve-lexer/src/span.rs` | Span type — start/end positions for diagnostics |
| `neve-parser/src/lib.rs` | Parser entry + Pratt parser + `parse_module()` |
| `neve-parser/src/expr.rs` | Expression parsing sub-functions |
| `neve-parser/src/pattern.rs` | Pattern parsing (match arms, let bindings) |
| `neve-syntax/src/expr.rs` | Core `Expr` / `ExprKind` / `MatchArm` types |
| `neve-syntax/src/item.rs` | Item, LetBinding, FnDef, TypeDef |
| `tests/parser.rs` | 220+ parser golden/integration tests |

## Testing

- **Golden tests**: Parse source, compare formatted AST output to `.txt` baseline
- **Integration tests**: `tests/parser.rs` — 220+ tests covering all syntax forms
- **Remaining gaps (2)**: Unicode `\u{...}` in chars, shebang at parser level
