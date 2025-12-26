//! Error codes for Neve diagnostics.

/// Error codes for categorizing diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Lexer errors (E0001 - E0099)
    UnexpectedCharacter,
    UnterminatedString,
    UnterminatedComment,
    InvalidEscape,
    InvalidNumber,

    // Parser errors (E0100 - E0199)
    UnexpectedToken,
    ExpectedExpression,
    ExpectedPattern,
    ExpectedType,
    UnclosedDelimiter,
    MissingSemicolon,

    // Type errors (E0200 - E0299)
    TypeMismatch,
    UnboundVariable,
    UnboundType,
    InfiniteType,
    NotAFunction,
    WrongArity,
    MissingField,
    UnknownField,
    TraitNotImplemented,
    MissingMethod,
    MissingAssocType,
    IfBranchMismatch,
    MatchArmMismatch,
    ReturnTypeMismatch,
    ArgumentTypeMismatch,
    BinaryOpTypeMismatch,
    UnaryOpTypeMismatch,
    CannotInferType,
    RecursiveType,
    AmbiguousType,

    // Eval errors (E0300 - E0399)
    DivisionByZero,
    AssertionFailed,
    PatternMatchFailed,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            // Lexer
            ErrorCode::UnexpectedCharacter => "E0001",
            ErrorCode::UnterminatedString => "E0002",
            ErrorCode::UnterminatedComment => "E0003",
            ErrorCode::InvalidEscape => "E0004",
            ErrorCode::InvalidNumber => "E0005",

            // Parser
            ErrorCode::UnexpectedToken => "E0100",
            ErrorCode::ExpectedExpression => "E0101",
            ErrorCode::ExpectedPattern => "E0102",
            ErrorCode::ExpectedType => "E0103",
            ErrorCode::UnclosedDelimiter => "E0104",
            ErrorCode::MissingSemicolon => "E0105",

            // Type
            ErrorCode::TypeMismatch => "E0200",
            ErrorCode::UnboundVariable => "E0201",
            ErrorCode::UnboundType => "E0202",
            ErrorCode::InfiniteType => "E0203",
            ErrorCode::NotAFunction => "E0204",
            ErrorCode::WrongArity => "E0205",
            ErrorCode::MissingField => "E0206",
            ErrorCode::UnknownField => "E0207",
            ErrorCode::TraitNotImplemented => "E0208",
            ErrorCode::MissingMethod => "E0209",
            ErrorCode::MissingAssocType => "E0210",
            ErrorCode::IfBranchMismatch => "E0211",
            ErrorCode::MatchArmMismatch => "E0212",
            ErrorCode::ReturnTypeMismatch => "E0213",
            ErrorCode::ArgumentTypeMismatch => "E0214",
            ErrorCode::BinaryOpTypeMismatch => "E0215",
            ErrorCode::UnaryOpTypeMismatch => "E0216",
            ErrorCode::CannotInferType => "E0217",
            ErrorCode::RecursiveType => "E0218",
            ErrorCode::AmbiguousType => "E0219",

            // Eval
            ErrorCode::DivisionByZero => "E0300",
            ErrorCode::AssertionFailed => "E0301",
            ErrorCode::PatternMatchFailed => "E0302",
        }
    }

    /// Get a human-readable description of the error.
    pub fn description(&self) -> &'static str {
        match self {
            // Lexer
            ErrorCode::UnexpectedCharacter => "unexpected character in input",
            ErrorCode::UnterminatedString => "string literal is not terminated",
            ErrorCode::UnterminatedComment => "comment is not terminated",
            ErrorCode::InvalidEscape => "invalid escape sequence in string",
            ErrorCode::InvalidNumber => "invalid number literal",

            // Parser
            ErrorCode::UnexpectedToken => "unexpected token",
            ErrorCode::ExpectedExpression => "expected an expression",
            ErrorCode::ExpectedPattern => "expected a pattern",
            ErrorCode::ExpectedType => "expected a type",
            ErrorCode::UnclosedDelimiter => "unclosed delimiter",
            ErrorCode::MissingSemicolon => "missing semicolon",

            // Type
            ErrorCode::TypeMismatch => "mismatched types",
            ErrorCode::UnboundVariable => "cannot find value in this scope",
            ErrorCode::UnboundType => "cannot find type in this scope",
            ErrorCode::InfiniteType => "cannot construct infinite type",
            ErrorCode::NotAFunction => "expected a function, found a different type",
            ErrorCode::WrongArity => "wrong number of arguments",
            ErrorCode::MissingField => "missing field in record",
            ErrorCode::UnknownField => "unknown field in record",
            ErrorCode::TraitNotImplemented => "trait is not implemented for type",
            ErrorCode::MissingMethod => "missing required method in trait implementation",
            ErrorCode::MissingAssocType => "missing required associated type in trait implementation",
            ErrorCode::IfBranchMismatch => "if and else branches have incompatible types",
            ErrorCode::MatchArmMismatch => "match arms have incompatible types",
            ErrorCode::ReturnTypeMismatch => "return type does not match function signature",
            ErrorCode::ArgumentTypeMismatch => "argument type does not match parameter type",
            ErrorCode::BinaryOpTypeMismatch => "binary operator cannot be applied to these types",
            ErrorCode::UnaryOpTypeMismatch => "unary operator cannot be applied to this type",
            ErrorCode::CannotInferType => "cannot infer type",
            ErrorCode::RecursiveType => "recursive type detected",
            ErrorCode::AmbiguousType => "type is ambiguous",

            // Eval
            ErrorCode::DivisionByZero => "division by zero",
            ErrorCode::AssertionFailed => "assertion failed",
            ErrorCode::PatternMatchFailed => "pattern matching failed",
        }
    }

    /// Get a suggested fix for the error, if available.
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            ErrorCode::UnterminatedString => Some("add a closing quote `\"` to terminate the string"),
            ErrorCode::UnterminatedComment => Some("add `*/` to close the comment"),
            ErrorCode::MissingSemicolon => Some("add `;` at the end of the statement"),
            ErrorCode::UnclosedDelimiter => Some("add the matching closing delimiter"),
            ErrorCode::UnboundVariable => Some("check the spelling or ensure the variable is in scope"),
            ErrorCode::UnboundType => Some("check the spelling or import the type"),
            ErrorCode::WrongArity => Some("check the function signature for the expected number of arguments"),
            ErrorCode::MissingField => Some("add the missing field to the record"),
            ErrorCode::MissingMethod => Some("implement all required methods for the trait"),
            ErrorCode::MissingAssocType => Some("specify all required associated types in the impl block"),
            _ => None,
        }
    }
}
