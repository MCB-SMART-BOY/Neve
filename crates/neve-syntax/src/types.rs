//! Type AST nodes.

use neve_common::Span;
use crate::Ident;

/// A type expression.
#[derive(Debug, Clone)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

impl Type {
    pub fn new(kind: TypeKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    /// A named type `Int`, `String`, `List<T>`
    Named {
        path: Vec<Ident>,
        args: Vec<Type>,
    },

    /// A function type `A -> B`
    Function {
        params: Vec<Type>,
        result: Box<Type>,
    },

    /// A tuple type `(A, B, C)`
    Tuple(Vec<Type>),

    /// A record type `#{ name: String, age: Int }`
    Record(Vec<RecordTypeField>),

    /// Unit type `()`
    Unit,

    /// Type variable (for inference)
    Infer,
}

/// A field in a record type.
#[derive(Debug, Clone)]
pub struct RecordTypeField {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}
