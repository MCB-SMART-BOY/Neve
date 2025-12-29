//! Semantic token generation for syntax highlighting.
//! 用于语法高亮的语义 token 生成。
//!
//! Converts lexer tokens to LSP semantic tokens for IDE highlighting.
//! 将词法 token 转换为用于 IDE 高亮的 LSP 语义 token。

use neve_lexer::{Token, TokenKind};
use tower_lsp::lsp_types::SemanticToken;

/// Token type indices (must match the legend in capabilities).
/// Token 类型索引（必须与 capabilities 中的 legend 匹配）。
pub mod token_types {
    /// Keyword. / 关键字。
    pub const KEYWORD: u32 = 0;
    /// Variable. / 变量。
    pub const VARIABLE: u32 = 1;
    /// Function. / 函数。
    pub const FUNCTION: u32 = 2;
    /// Type. / 类型。
    pub const TYPE: u32 = 3;
    /// String. / 字符串。
    pub const STRING: u32 = 4;
    /// Number. / 数字。
    pub const NUMBER: u32 = 5;
    /// Comment. / 注释。
    pub const COMMENT: u32 = 6;
    /// Operator. / 运算符。
    pub const OPERATOR: u32 = 7;
    /// Parameter. / 参数。
    pub const PARAMETER: u32 = 8;
    /// Property. / 属性。
    pub const PROPERTY: u32 = 9;
}

/// Token modifier bit flags.
/// Token 修饰符位标志。
pub mod token_modifiers {
    /// Declaration. / 声明。
    pub const DECLARATION: u32 = 1 << 0;
    /// Definition. / 定义。
    pub const DEFINITION: u32 = 1 << 1;
    /// Readonly. / 只读。
    pub const READONLY: u32 = 1 << 2;
}

/// Generate semantic tokens from lexer tokens.
/// 从词法 token 生成语义 token。
///
/// This is the basic version without context awareness.
/// For more accurate highlighting, use `generate_semantic_tokens_with_context`.
///
/// 这是不具备上下文感知的基本版本。
/// 要获得更准确的高亮，请使用 `generate_semantic_tokens_with_context`。
pub fn generate_semantic_tokens(tokens: &[Token], source: &str) -> Vec<SemanticToken> {
    let mut result = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_col = 0u32;

    for token in tokens {
        if let Some((token_type, modifiers)) = classify_token(token) {
            let start: usize = token.span.start.into();
            let (line, col) = offset_to_line_col(source, start);
            let length = token.span.len() as u32;

            let delta_line = line - prev_line;
            let delta_col = if delta_line == 0 { col - prev_col } else { col };

            result.push(SemanticToken {
                delta_line,
                delta_start: delta_col,
                length,
                token_type,
                token_modifiers_bitset: modifiers,
            });

            prev_line = line;
            prev_col = col;
        }
    }

    result
}

/// Get the token type for a parameter.
/// 获取参数的 token 类型。
///
/// This is useful when we know from context that an identifier is a parameter.
/// 当我们从上下文中知道标识符是参数时，这很有用。
#[inline]
pub fn parameter_token_type() -> u32 {
    token_types::PARAMETER
}

/// Get the token type for a comment.
/// 获取注释的 token 类型。
///
/// Reserved for future use when comments are preserved in the token stream.
/// 保留供将来在 token 流中保留注释时使用。
#[inline]
pub fn comment_token_type() -> u32 {
    token_types::COMMENT
}

/// Context for token classification (tracks what we've seen before).
/// Token 分类的上下文（跟踪我们之前看到的内容）。
#[derive(Default)]
struct ClassifyContext {
    /// Previous token was `fn` keyword. / 前一个 token 是 `fn` 关键字。
    after_fn: bool,
    /// Previous token was `let` keyword. / 前一个 token 是 `let` 关键字。
    after_let: bool,
    /// Previous token was a dot (field/property access). / 前一个 token 是点（字段/属性访问）。
    after_dot: bool,
}

/// Generate semantic tokens with context awareness.
/// 使用上下文感知生成语义 token。
pub fn generate_semantic_tokens_with_context(tokens: &[Token], source: &str) -> Vec<SemanticToken> {
    let mut result = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_col = 0u32;
    let mut ctx = ClassifyContext::default();

    for token in tokens {
        if let Some((token_type, modifiers)) = classify_token_with_context(token, &ctx) {
            let start: usize = token.span.start.into();
            let (line, col) = offset_to_line_col(source, start);
            let length = token.span.len() as u32;

            let delta_line = line - prev_line;
            let delta_col = if delta_line == 0 { col - prev_col } else { col };

            result.push(SemanticToken {
                delta_line,
                delta_start: delta_col,
                length,
                token_type,
                token_modifiers_bitset: modifiers,
            });

            prev_line = line;
            prev_col = col;
        }

        // Update context for next token
        // 更新下一个 token 的上下文
        ctx.after_fn = matches!(token.kind, TokenKind::Fn);
        ctx.after_let = matches!(token.kind, TokenKind::Let);
        ctx.after_dot = matches!(token.kind, TokenKind::Dot | TokenKind::QuestionDot);
    }

    result
}

/// Classify a token with context awareness.
/// 使用上下文感知分类 token。
fn classify_token_with_context(token: &Token, ctx: &ClassifyContext) -> Option<(u32, u32)> {
    let (token_type, modifiers) = match &token.kind {
        // Keywords / 关键字
        TokenKind::Let
        | TokenKind::Fn
        | TokenKind::If
        | TokenKind::Then
        | TokenKind::Else
        | TokenKind::Match
        | TokenKind::Type
        | TokenKind::Struct
        | TokenKind::Enum
        | TokenKind::Trait
        | TokenKind::Impl
        | TokenKind::Import
        | TokenKind::Pub
        | TokenKind::Lazy
        | TokenKind::As
        | TokenKind::SelfLower
        | TokenKind::Super => (token_types::KEYWORD, 0),

        // Literals / 字面量
        TokenKind::Int(_) | TokenKind::Float(_) => (token_types::NUMBER, 0),
        TokenKind::String(_) | TokenKind::Char(_) => (token_types::STRING, 0),
        TokenKind::True | TokenKind::False => (token_types::KEYWORD, 0),

        // Identifiers - use context to determine type
        // 标识符 - 使用上下文确定类型
        TokenKind::Ident(name) => {
            if ctx.after_fn {
                // Function definition / 函数定义
                (token_types::FUNCTION, token_modifiers::DEFINITION)
            } else if ctx.after_let {
                // Variable definition (readonly in Neve since it's immutable)
                // 变量定义（在 Neve 中是只读的，因为它是不可变的）
                (
                    token_types::VARIABLE,
                    token_modifiers::DECLARATION | token_modifiers::READONLY,
                )
            } else if ctx.after_dot {
                // Property/field access / 属性/字段访问
                (token_types::PROPERTY, 0)
            } else if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                // Type name (starts with uppercase) / 类型名（以大写字母开头）
                (token_types::TYPE, 0)
            } else {
                // Regular variable / 普通变量
                (token_types::VARIABLE, 0)
            }
        }

        // Note: Comments are skipped by the lexer and don't appear as tokens.
        // The COMMENT token type is available for future use if we add
        // comment preservation to the lexer.
        // 注意：注释被词法分析器跳过，不会作为 token 出现。
        // COMMENT token 类型保留供将来使用，如果我们在词法分析器中添加注释保留。

        // Operators / 运算符
        TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::Caret
        | TokenKind::Eq
        | TokenKind::EqEq
        | TokenKind::BangEq
        | TokenKind::Lt
        | TokenKind::LtEq
        | TokenKind::Gt
        | TokenKind::GtEq
        | TokenKind::AndAnd
        | TokenKind::OrOr
        | TokenKind::Bang
        | TokenKind::Pipe
        | TokenKind::PipeGt
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::PlusPlus
        | TokenKind::SlashSlash
        | TokenKind::Question
        | TokenKind::QuestionQuestion
        | TokenKind::QuestionDot => (token_types::OPERATOR, 0),

        // Skip punctuation, delimiters, and other tokens
        // 跳过标点符号、分隔符和其他 token
        _ => return None,
    };

    Some((token_type, modifiers))
}

/// Classify a token into a semantic token type.
/// 将 token 分类为语义 token 类型。
fn classify_token(token: &Token) -> Option<(u32, u32)> {
    classify_token_with_context(token, &ClassifyContext::default())
}

/// Convert byte offset to line and column.
/// 将字节偏移量转换为行和列。
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;

    for (i, c) in source.chars().enumerate() {
        if i == offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}
