//! Semantic token generation for syntax highlighting.

use neve_lexer::{Token, TokenKind};
use tower_lsp::lsp_types::SemanticToken;

/// Token type indices (must match the legend in capabilities).
pub mod token_types {
    pub const KEYWORD: u32 = 0;
    pub const VARIABLE: u32 = 1;
    pub const FUNCTION: u32 = 2;
    pub const TYPE: u32 = 3;
    pub const STRING: u32 = 4;
    pub const NUMBER: u32 = 5;
    pub const COMMENT: u32 = 6;
    pub const OPERATOR: u32 = 7;
    pub const PARAMETER: u32 = 8;
    pub const PROPERTY: u32 = 9;
}

/// Token modifier bit flags.
pub mod token_modifiers {
    pub const DECLARATION: u32 = 1 << 0;
    pub const DEFINITION: u32 = 1 << 1;
    pub const READONLY: u32 = 1 << 2;
}

/// Generate semantic tokens from lexer tokens.
/// This is the basic version without context awareness.
/// For more accurate highlighting, use `generate_semantic_tokens_with_context`.
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
/// This is useful when we know from context that an identifier is a parameter.
#[inline]
pub fn parameter_token_type() -> u32 {
    token_types::PARAMETER
}

/// Get the token type for a comment.
/// Reserved for future use when comments are preserved in the token stream.
#[inline]
pub fn comment_token_type() -> u32 {
    token_types::COMMENT
}

/// Context for token classification (tracks what we've seen before).
#[derive(Default)]
struct ClassifyContext {
    /// Previous token was `fn` keyword
    after_fn: bool,
    /// Previous token was `let` keyword
    after_let: bool,
    /// Previous token was a dot (field/property access)
    after_dot: bool,
}

/// Generate semantic tokens with context awareness.
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
        ctx.after_fn = matches!(token.kind, TokenKind::Fn);
        ctx.after_let = matches!(token.kind, TokenKind::Let);
        ctx.after_dot = matches!(token.kind, TokenKind::Dot | TokenKind::QuestionDot);
    }

    result
}

/// Classify a token with context awareness.
fn classify_token_with_context(token: &Token, ctx: &ClassifyContext) -> Option<(u32, u32)> {
    let (token_type, modifiers) = match &token.kind {
        // Keywords
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

        // Literals
        TokenKind::Int(_) | TokenKind::Float(_) => (token_types::NUMBER, 0),
        TokenKind::String(_) | TokenKind::Char(_) => (token_types::STRING, 0),
        TokenKind::True | TokenKind::False => (token_types::KEYWORD, 0),

        // Identifiers - use context to determine type
        TokenKind::Ident(name) => {
            if ctx.after_fn {
                // Function definition
                (token_types::FUNCTION, token_modifiers::DEFINITION)
            } else if ctx.after_let {
                // Variable definition (readonly in Neve since it's immutable)
                (
                    token_types::VARIABLE,
                    token_modifiers::DECLARATION | token_modifiers::READONLY,
                )
            } else if ctx.after_dot {
                // Property/field access
                (token_types::PROPERTY, 0)
            } else if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                // Type name (starts with uppercase)
                (token_types::TYPE, 0)
            } else {
                // Regular variable
                (token_types::VARIABLE, 0)
            }
        }

        // Note: Comments are skipped by the lexer and don't appear as tokens.
        // The COMMENT token type is available for future use if we add
        // comment preservation to the lexer.

        // Operators
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
        _ => return None,
    };

    Some((token_type, modifiers))
}

/// Classify a token into a semantic token type.
fn classify_token(token: &Token) -> Option<(u32, u32)> {
    classify_token_with_context(token, &ClassifyContext::default())
}

/// Convert byte offset to line and column.
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
