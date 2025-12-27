//! Expression AST nodes.

use neve_common::Span;
use crate::{Ident, Pattern, Type};

/// An expression.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    /// Integer literal
    Int(i64),
    /// Float literal
    Float(f64),
    /// String literal
    String(String),
    /// Interpolated string `` `hello {name}` ``
    Interpolated(Vec<StringPart>),
    /// Character literal
    Char(char),
    /// Boolean literal
    Bool(bool),
    /// Unit `()`
    Unit,

    /// Variable reference
    Var(Ident),

    /// Record literal `#{ x = 1, y = 2 }`
    Record(Vec<RecordField>),

    /// Record update `#{ record | x = 1 }`
    RecordUpdate {
        base: Box<Expr>,
        fields: Vec<RecordField>,
    },

    /// List literal `[1, 2, 3]`
    List(Vec<Expr>),

    /// List comprehension `[x * 2 | x <- xs, x > 0]`
    ListComp {
        body: Box<Expr>,
        generators: Vec<Generator>,
    },

    /// Tuple `(a, b, c)`
    Tuple(Vec<Expr>),

    /// Lambda `fn(x) x + 1`
    Lambda {
        params: Vec<LambdaParam>,
        body: Box<Expr>,
    },

    /// Function call `f(x, y)`
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    /// Method call `x.foo(y)`
    MethodCall {
        receiver: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
    },

    /// Field access `x.field`
    Field {
        base: Box<Expr>,
        field: Ident,
    },

    /// Tuple index `t.0`
    TupleIndex {
        base: Box<Expr>,
        index: u32,
    },

    /// Safe field access `x?.field`
    SafeField {
        base: Box<Expr>,
        field: Ident,
    },

    /// Index `xs[0]`
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },

    /// Binary operation `a + b`
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Unary operation `!a` or `-a`
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },

    /// Error propagation `expr?`
    Try(Box<Expr>),

    /// Default value `expr ?? default`
    Coalesce {
        value: Box<Expr>,
        default: Box<Expr>,
    },

    /// If expression `if cond then a else b`
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },

    /// Match expression
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// Block `{ let x = 1; x + 1 }`
    Block {
        stmts: Vec<Stmt>,
        expr: Option<Box<Expr>>,
    },

    /// Let expression `let x = 1; x + 1`
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        value: Box<Expr>,
        body: Box<Expr>,
    },

    /// Lazy expression `lazy expr`
    Lazy(Box<Expr>),

    /// Path expression `std.list.map`
    Path(Vec<Ident>),
}

/// A record field `name = value`.
#[derive(Debug, Clone)]
pub struct RecordField {
    pub name: Ident,
    pub value: Option<Expr>,
    pub span: Span,
}

/// A generator in a list comprehension.
#[derive(Debug, Clone)]
pub struct Generator {
    pub pattern: Pattern,
    pub iter: Expr,
    pub condition: Option<Expr>,
    pub span: Span,
}

/// A lambda parameter.
#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub pattern: Pattern,
    pub ty: Option<Type>,
    pub span: Span,
}

/// A match arm.
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// A statement in a block.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    /// Let binding `let x = 1;`
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        value: Expr,
    },
    /// Expression statement `expr;`
    Expr(Expr),
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Mod,    // %
    Pow,    // ^

    // Comparison
    Eq,     // ==
    Ne,     // !=
    Lt,     // <
    Le,     // <=
    Gt,     // >
    Ge,     // >=

    // Logical
    And,    // &&
    Or,     // ||

    // Other
    Concat, // ++
    Merge,  // //
    Pipe,   // |>
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,    // -
    Not,    // !
}

/// A part of an interpolated string.
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Literal string part
    Literal(String),
    /// Interpolated expression `{expr}`
    Expr(Expr),
}
