//! Type AST nodes.
//! 类型 AST 节点。

use crate::Ident;
use neve_common::Span;

/// A type expression.
/// 类型表达式。
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

/// Type kind.
/// 类型种类。
#[derive(Debug, Clone)]
pub enum TypeKind {
    /// A named type `Int`, `String`, `List<T>` / 命名类型
    Named { path: Vec<Ident>, args: Vec<Type> },

    /// A function type `A -> B` / 函数类型
    Function {
        params: Vec<Type>,
        result: Box<Type>,
    },

    /// A tuple type `(A, B, C)` / 元组类型
    Tuple(Vec<Type>),

    /// A record type `#{ name: String, age: Int }` / 记录类型
    Record(Vec<RecordTypeField>),

    /// Unit type `()` / 单元类型
    Unit,

    /// Type variable (for inference) / 类型变量（用于推断）
    Infer,
}

/// A field in a record type.
/// 记录类型中的字段。
#[derive(Debug, Clone)]
pub struct RecordTypeField {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}
