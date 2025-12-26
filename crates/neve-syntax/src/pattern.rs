//! Pattern AST nodes.

use neve_common::Span;
use crate::Ident;

/// A pattern for matching.
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

#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Wildcard `_`
    Wildcard,

    /// Variable binding `x`
    Var(Ident),

    /// Literal pattern `42`, `"hello"`, `true`
    Literal(LiteralPattern),

    /// Tuple pattern `(a, b, c)`
    Tuple(Vec<Pattern>),

    /// List pattern `[a, b, c]`
    List(Vec<Pattern>),

    /// List with rest `[head, ..tail]`
    ListRest {
        init: Vec<Pattern>,
        rest: Option<Box<Pattern>>,
        tail: Vec<Pattern>,
    },

    /// Record pattern `#{ x, y = z }`
    Record {
        fields: Vec<RecordPatternField>,
        rest: bool,
    },

    /// Constructor pattern `Some(x)` or `None`
    Constructor {
        path: Vec<Ident>,
        args: Vec<Pattern>,
    },

    /// Or pattern `a | b`
    Or(Vec<Pattern>),

    /// Binding pattern `name @ pattern`
    Binding {
        name: Ident,
        pattern: Box<Pattern>,
    },
}

/// A literal in a pattern.
#[derive(Debug, Clone)]
pub enum LiteralPattern {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
}

/// A field in a record pattern.
#[derive(Debug, Clone)]
pub struct RecordPatternField {
    pub name: Ident,
    pub pattern: Option<Pattern>,
    pub span: Span,
}
