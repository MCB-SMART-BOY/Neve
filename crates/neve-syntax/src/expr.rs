//! Expression AST nodes.
//! 表达式 AST 节点。

use crate::{Ident, Pattern, Type};
use neve_common::Span;

/// An expression.
/// 表达式。
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

/// Expression kind.
/// 表达式类型。
#[derive(Debug, Clone)]
pub enum ExprKind {
    /// Integer literal / 整数字面量
    Int(i64),
    /// Float literal / 浮点数字面量
    Float(f64),
    /// String literal / 字符串字面量
    String(String),
    /// Interpolated string `` `hello {name}` `` / 插值字符串
    Interpolated(Vec<StringPart>),
    /// Character literal / 字符字面量
    Char(char),
    /// Boolean literal / 布尔字面量
    Bool(bool),
    /// Unit `()` / 单元值
    Unit,

    /// Variable reference / 变量引用
    Var(Ident),

    /// Record literal `#{ x = 1, y = 2 }` / 记录字面量
    Record(Vec<RecordField>),

    /// Record update `#{ record | x = 1 }` / 记录更新
    RecordUpdate {
        base: Box<Expr>,
        fields: Vec<RecordField>,
    },

    /// List literal `[1, 2, 3]` / 列表字面量
    List(Vec<Expr>),

    /// List comprehension `[x * 2 | x <- xs, x > 0]` / 列表推导
    ListComp {
        body: Box<Expr>,
        generators: Vec<Generator>,
    },

    /// Tuple `(a, b, c)` / 元组
    Tuple(Vec<Expr>),

    /// Lambda `fn(x) x + 1` / Lambda 表达式
    Lambda {
        params: Vec<LambdaParam>,
        body: Box<Expr>,
    },

    /// Function call `f(x, y)` / 函数调用
    Call { func: Box<Expr>, args: Vec<Expr> },

    /// Method call `x.foo(y)` / 方法调用
    MethodCall {
        receiver: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
    },

    /// Field access `x.field` / 字段访问
    Field { base: Box<Expr>, field: Ident },

    /// Tuple index `t.0` / 元组索引
    TupleIndex { base: Box<Expr>, index: u32 },

    /// Safe field access `x?.field` / 安全字段访问
    SafeField { base: Box<Expr>, field: Ident },

    /// Index `xs[0]` / 索引访问
    Index { base: Box<Expr>, index: Box<Expr> },

    /// Binary operation `a + b` / 二元运算
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Unary operation `!a` or `-a` / 一元运算
    Unary { op: UnaryOp, operand: Box<Expr> },

    /// Error propagation `expr?` / 错误传播
    Try(Box<Expr>),

    /// Default value `expr ?? default` / 默认值
    Coalesce {
        value: Box<Expr>,
        default: Box<Expr>,
    },

    /// If expression `if cond then a else b` / 条件表达式
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },

    /// Match expression / 模式匹配表达式
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    /// Block `{ let x = 1; x + 1 }` / 块表达式
    Block {
        stmts: Vec<Stmt>,
        expr: Option<Box<Expr>>,
    },

    /// Let expression `let x = 1; x + 1` / let 表达式
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        value: Box<Expr>,
        body: Box<Expr>,
    },

    /// Lazy expression `lazy expr` / 惰性表达式
    Lazy(Box<Expr>),

    /// Dotted path expression `std.list.map` / 点路径表达式
    Path(Vec<Ident>),

    /// File system path literal `./foo`, `/bar`, `../baz` / 文件路径字面量
    PathLit(String),
}

/// A record field `name = value`.
/// 记录字段 `name = value`。
#[derive(Debug, Clone)]
pub struct RecordField {
    pub name: Ident,
    pub value: Option<Expr>,
    pub span: Span,
}

/// A generator in a list comprehension.
/// 列表推导中的生成器。
#[derive(Debug, Clone)]
pub struct Generator {
    pub pattern: Pattern,
    pub iter: Expr,
    pub condition: Option<Expr>,
    pub span: Span,
}

/// A lambda parameter.
/// Lambda 参数。
#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub pattern: Pattern,
    pub ty: Option<Type>,
    pub span: Span,
}

/// A match arm.
/// 匹配分支。
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

/// A statement in a block.
/// 块中的语句。
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

/// Statement kind.
/// 语句类型。
#[derive(Debug, Clone)]
pub enum StmtKind {
    /// Let binding `let x = 1;` / let 绑定
    Let {
        pattern: Pattern,
        ty: Option<Type>,
        value: Expr,
    },
    /// Expression statement `expr;` / 表达式语句
    Expr(Expr),
}

/// Binary operators.
/// 二元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic 算术运算
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %
    Pow, // ^

    // Comparison 比较运算
    Eq, // ==
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=

    // Logical 逻辑运算
    And, // &&
    Or,  // ||

    // Other 其他
    Concat, // ++
    Merge,  // //
    Pipe,   // |>
}

/// Unary operators.
/// 一元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg, // - 取负
    Not, // ! 取反
}

/// A part of an interpolated string.
/// 插值字符串的一部分。
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Literal string part / 字面字符串部分
    Literal(String),
    /// Interpolated expression `{expr}` / 插值表达式
    Expr(Expr),
}
