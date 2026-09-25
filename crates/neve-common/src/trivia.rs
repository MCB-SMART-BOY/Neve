//! Source trivia retained outside the token stream.
//! 词法 token 流之外保留的源码 trivia。

use crate::Span;

/// The kind of a source comment.
/// 源码注释的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommentKind {
    /// A comment that ends at the next newline.
    /// 延续到下一个换行符的注释。
    Line,
    /// A delimited comment that may span multiple lines.
    /// 可跨多行的定界注释。
    Block,
}

/// A source comment retained for formatting and tooling.
/// 为格式化和工具链保留的源码注释。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The comment kind.
    /// 注释类型。
    pub kind: CommentKind,
    /// The byte span occupied by the comment, excluding following whitespace.
    /// 注释占用的字节范围，不包含后续空白。
    pub span: Span,
    /// The exact source text covered by `span`.
    /// `span` 覆盖的原始源码文本。
    pub text: String,
    /// Whether only whitespace precedes the comment on its source line.
    /// 注释前是否只有空白字符。
    pub is_line_start: bool,
}

impl Comment {
    /// Create retained comment trivia.
    /// 创建保留的注释 trivia。
    pub fn new(kind: CommentKind, span: Span, text: String, is_line_start: bool) -> Self {
        Self {
            kind,
            span,
            text,
            is_line_start,
        }
    }
}
