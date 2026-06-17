//! Error codes for Neve diagnostics.
//! Neve 诊断的错误代码。

/// Error codes for categorizing diagnostics.
/// 用于分类诊断的错误代码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // ===== Lexer errors (E0001 - E0099) 词法错误 =====
    UnexpectedCharacter,
    UnterminatedString,
    UnterminatedComment,
    InvalidEscape,
    InvalidNumber,

    // ===== Parser errors (E0100 - E0199) 语法错误 =====
    UnexpectedToken,
    ExpectedExpression,
    ExpectedPattern,
    ExpectedType,
    UnclosedDelimiter,
    MissingSemicolon,
    ExpectedIdentifier,
    InvalidTupleIndex,

    // ===== Type errors (E0200 - E0299) 类型错误 =====
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
    NonExhaustiveMatch,
    UnreachablePattern,
    PrivateAccess,
    CyclicDependency,
    UnknownMethod,
    UnusedVariable,
    RedundantAnnotation,

    // ===== Eval errors (E0300 - E0399) 求值错误 =====
    DivisionByZero,
    AssertionFailed,
    PatternMatchFailed,
    EvalTypeError,
    EvalNotAFunction,
    EvalWrongArity,
    EvalUnboundVariable,

    // ===== Module errors (E0400 - E0499) 模块错误 =====
    ModuleNotFound,
    ModuleParseError,
    CircularImport,
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
            ErrorCode::ExpectedIdentifier => "E0106",
            ErrorCode::InvalidTupleIndex => "E0107",

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
            ErrorCode::NonExhaustiveMatch => "E0220",
            ErrorCode::UnreachablePattern => "E0221",
            ErrorCode::PrivateAccess => "E0222",
            ErrorCode::CyclicDependency => "E0223",
            ErrorCode::UnknownMethod => "E0224",
            ErrorCode::UnusedVariable => "E0225",
            ErrorCode::RedundantAnnotation => "E0226",

            // Eval
            ErrorCode::DivisionByZero => "E0300",
            ErrorCode::AssertionFailed => "E0301",
            ErrorCode::PatternMatchFailed => "E0302",
            ErrorCode::EvalTypeError => "E0303",
            ErrorCode::EvalNotAFunction => "E0304",
            ErrorCode::EvalWrongArity => "E0305",
            ErrorCode::EvalUnboundVariable => "E0306",

            // Module
            ErrorCode::ModuleNotFound => "E0400",
            ErrorCode::ModuleParseError => "E0401",
            ErrorCode::CircularImport => "E0402",
        }
    }

    /// Get a human-readable description of the error.
    /// 获取错误的可读描述。
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
            ErrorCode::ExpectedIdentifier => "expected an identifier",
            ErrorCode::InvalidTupleIndex => "invalid tuple index",

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
            ErrorCode::MissingAssocType => {
                "missing required associated type in trait implementation"
            }
            ErrorCode::IfBranchMismatch => "if and else branches have incompatible types",
            ErrorCode::MatchArmMismatch => "match arms have incompatible types",
            ErrorCode::ReturnTypeMismatch => "return type does not match function signature",
            ErrorCode::ArgumentTypeMismatch => "argument type does not match parameter type",
            ErrorCode::BinaryOpTypeMismatch => "binary operator cannot be applied to these types",
            ErrorCode::UnaryOpTypeMismatch => "unary operator cannot be applied to this type",
            ErrorCode::CannotInferType => "cannot infer type",
            ErrorCode::RecursiveType => "recursive type detected",
            ErrorCode::AmbiguousType => "type is ambiguous",
            ErrorCode::NonExhaustiveMatch => "match expression is not exhaustive",
            ErrorCode::UnreachablePattern => "unreachable pattern in match",
            ErrorCode::PrivateAccess => "cannot access private binding",
            ErrorCode::CyclicDependency => "cyclic dependency detected",
            ErrorCode::UnknownMethod => "cannot resolve method call on receiver type",
            ErrorCode::UnusedVariable => "unused variable",
            ErrorCode::RedundantAnnotation => "redundant type annotation",

            // Eval
            ErrorCode::DivisionByZero => "division by zero",
            ErrorCode::AssertionFailed => "assertion failed",
            ErrorCode::PatternMatchFailed => "pattern matching failed",
            ErrorCode::EvalTypeError => "runtime type error",
            ErrorCode::EvalNotAFunction => "expected a function at runtime",
            ErrorCode::EvalWrongArity => "wrong number of arguments at runtime",
            ErrorCode::EvalUnboundVariable => "unbound variable at runtime",

            // Module
            ErrorCode::ModuleNotFound => "module not found",
            ErrorCode::ModuleParseError => "parse error in imported module",
            ErrorCode::CircularImport => "circular import detected",
        }
    }

    /// Get a suggested fix for the error, if available.
    /// 获取错误的修复建议（如果有）。
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            ErrorCode::UnterminatedString => {
                Some("add a closing quote `\"` to terminate the string")
            }
            ErrorCode::UnterminatedComment => Some("add `-- --` to close the block comment"),
            ErrorCode::MissingSemicolon => Some("add `;` at the end of the statement"),
            ErrorCode::UnclosedDelimiter => Some("add the matching closing delimiter"),
            ErrorCode::ExpectedExpression => Some("add an expression here"),
            ErrorCode::ExpectedPattern => Some("add a pattern here"),
            ErrorCode::ExpectedType => Some("add a type annotation here"),
            ErrorCode::UnboundVariable => {
                Some("check the spelling or ensure the variable is in scope")
            }
            ErrorCode::UnboundType => Some("check the spelling or import the type"),
            ErrorCode::WrongArity => {
                Some("check the function signature for the expected number of arguments")
            }
            ErrorCode::MissingField => Some("add the missing field to the record"),
            ErrorCode::MissingMethod => Some("implement all required methods for the trait"),
            ErrorCode::MissingAssocType => {
                Some("specify all required associated types in the impl block")
            }
            ErrorCode::NonExhaustiveMatch => {
                Some("add patterns for all possible cases or use a wildcard `_` pattern")
            }
            ErrorCode::UnreachablePattern => {
                Some("remove the unreachable pattern or reorder the match arms")
            }
            ErrorCode::PrivateAccess => Some("make the binding accessible from the calling module"),
            ErrorCode::CyclicDependency => {
                Some("break the cycle by restructuring the dependencies")
            }
            ErrorCode::UnknownMethod => Some(
                "implement the method for the receiver type or define a matching callable fallback",
            ),
            ErrorCode::UnusedVariable => {
                Some("prefix the variable name with an underscore to suppress this warning")
            }
            ErrorCode::RedundantAnnotation => Some("remove the unnecessary type annotation"),
            ErrorCode::DivisionByZero => Some("ensure the divisor is non-zero before dividing"),
            ErrorCode::AssertionFailed => Some("check the condition or add a descriptive message"),
            ErrorCode::PatternMatchFailed => {
                Some("ensure all possible cases are covered at runtime")
            }
            ErrorCode::EvalTypeError => Some("add a type annotation or guard to narrow the type"),
            ErrorCode::EvalNotAFunction => Some("check that the call target is a function"),
            ErrorCode::EvalWrongArity => {
                Some("check the function definition for the expected number of arguments")
            }
            ErrorCode::EvalUnboundVariable => Some("define the variable before using it"),
            ErrorCode::ModuleNotFound => Some("check the module name or add it as a dependency"),
            ErrorCode::ModuleParseError => {
                Some("fix the parse errors in the imported module first")
            }
            ErrorCode::CircularImport => {
                Some("break the import cycle by restructuring the modules")
            }
            _ => None,
        }
    }

    /// Get an extended explanation for the error, suitable for `neve --explain`.
    /// 获取错误的扩展说明，适用于 `neve --explain`。
    pub fn extended_explanation(&self) -> Option<&'static str> {
        match self {
            // --- Type errors with extended explanations ---
            ErrorCode::TypeMismatch => Some(
                "A type mismatch occurs when the compiler expected one type but found another.\n\
                 \n\
                 Common causes:\n\
                 - Returning a value of the wrong type from a function\n\
                 - Passing an argument of the wrong type to a function\n\
                 - Assigning a value of the wrong type to a binding\n\
                 \n\
                 Check the types shown in the error message and ensure they match. \
                 If you need to convert between types, use conversion functions like \
                 `toInt`, `toFloat`, or `toString`.",
            ),
            ErrorCode::UnboundVariable => Some(
                "An unbound variable error means the compiler cannot find a value with \
                 the given name in the current scope.\n\
                 \n\
                 Common causes:\n\
                 - A typo in the variable name\n\
                 - The variable is defined in a different scope\n\
                 - Missing `use` import for a module member\n\
                 \n\
                 Check the spelling and ensure the variable is defined before use.",
            ),
            ErrorCode::InfiniteType => Some(
                "An infinite type error occurs when a type variable appears in its own \
                 type, creating a type that would be infinitely large.\n\
                 \n\
                 Example of what causes this:\n\
                 ```neve\n\
                 fn f(x) = x(x);  // x is applied to itself, creating ?T = ?T -> ?U\n\
                 ```\n\
                 \n\
                 This is usually caused by self-application or recursive definitions \
                 without a base case. Consider restructuring your code to avoid this pattern.",
            ),
            ErrorCode::NonExhaustiveMatch => Some(
                "A non-exhaustive match means the match expression does not cover all \
                 possible values of the matched type.\n\
                 \n\
                 Common causes:\n\
                 - Missing some enum variant arms\n\
                 - Missing a wildcard `_` pattern for unhandled cases\n\
                 \n\
                 Add the missing patterns or use `_` as a catch-all.",
            ),
            ErrorCode::UnreachablePattern => Some(
                "An unreachable pattern warning means a match arm will never be executed \
                 because a previous arm already matches all values this arm would match.\n\
                 \n\
                 This is often caused by:\n\
                 - A catch-all `_` pattern before more specific patterns\n\
                 - A variable pattern that shadows subsequent arms\n\
                 \n\
                 Remove the dead arm or reorder the match arms so more specific patterns \
                 come before less specific ones.",
            ),
            ErrorCode::CannotInferType => Some(
                "The compiler cannot infer the type of an expression from its context.\n\
                 \n\
                 This often happens with:\n\
                 - Empty collections without type annotations\n\
                 - Functions with no type annotations and no call sites\n\
                 - Ambiguous numeric literals\n\
                 \n\
                 Add a type annotation to help the compiler determine the intended type.",
            ),
            ErrorCode::AmbiguousType => Some(
                "The type of an expression is ambiguous — the compiler found multiple \
                 possible types and cannot determine which one is intended.\n\
                 \n\
                 Add an explicit type annotation to resolve the ambiguity.",
            ),

            // --- Parser errors ---
            ErrorCode::UnclosedDelimiter => Some(
                "An unclosed delimiter error means a pair delimiter — parentheses `()`, \
                 braces `{}`, or brackets `[]` — was opened but never closed.\n\
                 \n\
                 The error points to the opening delimiter. Check that every opening \
                 delimiter has a matching closing delimiter.",
            ),

            // --- Eval errors ---
            ErrorCode::DivisionByZero => Some(
                "Division by zero occurs when the right-hand operand of a division (`/`) \
                 or modulo (`%`) operation evaluates to zero.\n\
                 \n\
                 Always check that the divisor is non-zero before performing division. \
                 You can use an `if` guard:\n\
                 ```neve\n\
                 if divisor != 0 -> x / divisor else 0\n\
                 ```",
            ),
            ErrorCode::PatternMatchFailed => Some(
                "A runtime pattern match failure means the value being matched does not \
                 fit any of the patterns in the match expression.\n\
                 \n\
                 This differs from E0220 (non-exhaustive match) which is detected at \
                 compile time. This error occurs when the type system cannot guarantee \
                 exhaustiveness at compile time, such as with dynamic records.\n\
                 \n\
                 Add a catch-all pattern `_` to handle unexpected cases.",
            ),

            // --- Module errors ---
            ErrorCode::ModuleNotFound => Some(
                "The compiler cannot find the specified module.\n\
                 \n\
                 Check that:\n\
                 - The module name is spelled correctly\n\
                 - The module file exists in the expected location\n\
                 - The module is listed as a dependency\n\
                 \n\
                 Module paths correspond to file paths relative to the project root.",
            ),
            ErrorCode::CircularImport => Some(
                "A circular import occurs when two or more modules import each other, \
                 directly or indirectly.\n\
                 \n\
                 Example:\n\
                 - Module A imports B\n\
                 - Module B imports A\n\
                 \n\
                 Break the cycle by extracting shared definitions into a third module \
                 that both modules can import.",
            ),

            _ => None,
        }
    }

    /// Get the documentation URL for this error code.
    /// 获取此错误代码的文档链接。
    pub fn doc_url(&self) -> String {
        const BASE_URL: &str =
            "https://github.com/MCB-SMART-BOY/Neve/blob/master/docs/reference/diagnostics.md#";
        format!("{BASE_URL}{}", self.as_str())
    }
}

/// Look up an error code from its string representation.
/// 从其字符串表示中查找错误代码。
pub fn lookup_error_code(code_str: &str) -> Option<ErrorCode> {
    match code_str.to_uppercase().as_str() {
        "E0001" => Some(ErrorCode::UnexpectedCharacter),
        "E0002" => Some(ErrorCode::UnterminatedString),
        "E0003" => Some(ErrorCode::UnterminatedComment),
        "E0004" => Some(ErrorCode::InvalidEscape),
        "E0005" => Some(ErrorCode::InvalidNumber),
        "E0100" => Some(ErrorCode::UnexpectedToken),
        "E0101" => Some(ErrorCode::ExpectedExpression),
        "E0102" => Some(ErrorCode::ExpectedPattern),
        "E0103" => Some(ErrorCode::ExpectedType),
        "E0104" => Some(ErrorCode::UnclosedDelimiter),
        "E0105" => Some(ErrorCode::MissingSemicolon),
        "E0106" => Some(ErrorCode::ExpectedIdentifier),
        "E0107" => Some(ErrorCode::InvalidTupleIndex),
        "E0200" => Some(ErrorCode::TypeMismatch),
        "E0201" => Some(ErrorCode::UnboundVariable),
        "E0202" => Some(ErrorCode::UnboundType),
        "E0203" => Some(ErrorCode::InfiniteType),
        "E0204" => Some(ErrorCode::NotAFunction),
        "E0205" => Some(ErrorCode::WrongArity),
        "E0206" => Some(ErrorCode::MissingField),
        "E0207" => Some(ErrorCode::UnknownField),
        "E0208" => Some(ErrorCode::TraitNotImplemented),
        "E0209" => Some(ErrorCode::MissingMethod),
        "E0210" => Some(ErrorCode::MissingAssocType),
        "E0211" => Some(ErrorCode::IfBranchMismatch),
        "E0212" => Some(ErrorCode::MatchArmMismatch),
        "E0213" => Some(ErrorCode::ReturnTypeMismatch),
        "E0214" => Some(ErrorCode::ArgumentTypeMismatch),
        "E0215" => Some(ErrorCode::BinaryOpTypeMismatch),
        "E0216" => Some(ErrorCode::UnaryOpTypeMismatch),
        "E0217" => Some(ErrorCode::CannotInferType),
        "E0218" => Some(ErrorCode::RecursiveType),
        "E0219" => Some(ErrorCode::AmbiguousType),
        "E0220" => Some(ErrorCode::NonExhaustiveMatch),
        "E0221" => Some(ErrorCode::UnreachablePattern),
        "E0222" => Some(ErrorCode::PrivateAccess),
        "E0223" => Some(ErrorCode::CyclicDependency),
        "E0224" => Some(ErrorCode::UnknownMethod),
        "E0225" => Some(ErrorCode::UnusedVariable),
        "E0226" => Some(ErrorCode::RedundantAnnotation),
        "E0300" => Some(ErrorCode::DivisionByZero),
        "E0301" => Some(ErrorCode::AssertionFailed),
        "E0302" => Some(ErrorCode::PatternMatchFailed),
        "E0303" => Some(ErrorCode::EvalTypeError),
        "E0304" => Some(ErrorCode::EvalNotAFunction),
        "E0305" => Some(ErrorCode::EvalWrongArity),
        "E0306" => Some(ErrorCode::EvalUnboundVariable),
        "E0400" => Some(ErrorCode::ModuleNotFound),
        "E0401" => Some(ErrorCode::ModuleParseError),
        "E0402" => Some(ErrorCode::CircularImport),
        _ => None,
    }
}
