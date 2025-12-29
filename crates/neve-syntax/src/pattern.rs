//! Pattern AST nodes.
//! 模式 AST 节点。

use crate::Ident;
use neve_common::Span;

/// A pattern for matching.
/// 用于匹配的模式。
#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

impl Pattern {
    pub fn new(kind: PatternKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Pattern kind.
/// 模式类型。
#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Wildcard `_` / 通配符
    Wildcard,

    /// Variable binding `x` / 变量绑定
    Var(Ident),

    /// Literal pattern `42`, `"hello"`, `true` / 字面量模式
    Literal(LiteralPattern),

    /// Tuple pattern `(a, b, c)` / 元组模式
    Tuple(Vec<Pattern>),

    /// List pattern `[a, b, c]` / 列表模式
    List(Vec<Pattern>),

    /// List with rest `[head, ..tail]` / 带剩余的列表模式
    ListRest {
        init: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
        tail: Vec<Pattern>,
    },

    /// Record pattern `#{ x, y = z }` / 记录模式
    Record {
        fields: Vec<RecordPatternField>,
        rest: bool,
    },

    /// Constructor pattern `Some(x)` or `None` / 构造器模式
    Constructor {
        path: Vec<Ident>,
        args: Vec<Pattern>,
    },

    /// Or pattern `a | b` / 或模式
    Or(Vec<Pattern>),

    /// Binding pattern `name @ pattern` / 绑定模式
    Binding { name: Ident, pattern: Box<Pattern> },
}

/// A literal in a pattern.
/// 模式中的字面量。
#[derive(Debug, Clone)]
pub enum LiteralPattern {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
}

/// A field in a record pattern.
/// 记录模式中的字段。
#[derive(Debug, Clone)]
pub struct RecordPatternField {
    pub name: Ident,
    pub pattern: Option<Pattern>,
    pub span: Span,
}
