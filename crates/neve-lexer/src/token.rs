//! Token definitions for Neve.

use neve_common::Span;

/// A token with its kind and span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kind of a token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    /// Path literal (e.g., `./foo`, `/bar`, `../baz`)
    PathLit(String),

    // Interpolated string parts
    /// Start of interpolated string `` ` ``
    InterpolatedStart,
    /// String part in interpolated string (between { })
    InterpolatedPart(String),
    /// End of interpolated string `` ` ``
    InterpolatedEnd,
    /// Start of interpolation `{`
    InterpolationStart,
    /// End of interpolation `}`
    InterpolationEnd,

    // Identifiers
    Ident(String),

    // Keywords
    Let,
    Fn,
    Type,
    Struct,
    Enum,
    Trait,
    Impl,
    Pub,
    Import,
    As,
    SelfLower,  // self
    Super,      // super
    Crate,      // crate
    If,
    Then,
    Else,
    Match,
    Lazy,
    True,
    False,

    // Delimiters
    LParen,     // (
    RParen,     // )
    LBracket,   // [
    RBracket,   // ]
    LBrace,     // {
    RBrace,     // }
    HashLBrace, // #{

    // Operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Caret,      // ^
    Eq,         // =
    EqEq,       // ==
    BangEq,     // !=
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=
    AndAnd,     // &&
    OrOr,       // ||
    Bang,       // !
    PlusPlus,   // ++
    SlashSlash, // //
    QuestionQuestion, // ??
    QuestionDot,      // ?.
    Pipe,       // |
    PipeGt,     // |>
    Arrow,      // ->
    FatArrow,   // =>
    At,         // @
    DotDot,     // ..
    Question,   // ?

    // Punctuation
    Comma,      // ,
    Colon,      // :
    Semicolon,  // ;
    Dot,        // .

    // Special
    Eof,
    Error,
}

impl TokenKind {
    /// Returns true if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Let
                | TokenKind::Fn
                | TokenKind::Type
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Pub
                | TokenKind::Import
                | TokenKind::As
                | TokenKind::SelfLower
                | TokenKind::Super
                | TokenKind::Crate
                | TokenKind::If
                | TokenKind::Then
                | TokenKind::Else
                | TokenKind::Match
                | TokenKind::Lazy
                | TokenKind::True
                | TokenKind::False
        )
    }

    /// Returns the keyword for an identifier, if any.
    pub fn keyword_from_str(s: &str) -> Option<TokenKind> {
        match s {
            "let" => Some(TokenKind::Let),
            "fn" => Some(TokenKind::Fn),
            "type" => Some(TokenKind::Type),
            "struct" => Some(TokenKind::Struct),
            "enum" => Some(TokenKind::Enum),
            "trait" => Some(TokenKind::Trait),
            "impl" => Some(TokenKind::Impl),
            "pub" => Some(TokenKind::Pub),
            "import" => Some(TokenKind::Import),
            "as" => Some(TokenKind::As),
            "self" => Some(TokenKind::SelfLower),
            "super" => Some(TokenKind::Super),
            "crate" => Some(TokenKind::Crate),
            "if" => Some(TokenKind::If),
            "then" => Some(TokenKind::Then),
            "else" => Some(TokenKind::Else),
            "match" => Some(TokenKind::Match),
            "lazy" => Some(TokenKind::Lazy),
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            _ => None,
        }
    }
}
