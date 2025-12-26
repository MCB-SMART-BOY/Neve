//! Error recovery strategies for the parser.
//!
//! This module provides utilities for recovering from parse errors
//! so the parser can continue and report multiple errors.

use neve_lexer::TokenKind;

/// Tokens that typically start a new statement/item.
pub const STMT_STARTS: &[TokenKind] = &[
    TokenKind::Let,
    TokenKind::Fn,
    TokenKind::Type,
    TokenKind::Struct,
    TokenKind::Enum,
    TokenKind::Trait,
    TokenKind::Impl,
    TokenKind::Import,
    TokenKind::Pub,
];

/// Tokens that typically end a statement.
pub const STMT_ENDS: &[TokenKind] = &[
    TokenKind::Semicolon,
    TokenKind::RBrace,
];

/// Tokens that are synchronization points.
pub const SYNC_TOKENS: &[TokenKind] = &[
    TokenKind::Semicolon,
    TokenKind::RBrace,
    TokenKind::Let,
    TokenKind::Fn,
    TokenKind::Type,
    TokenKind::Struct,
    TokenKind::Enum,
    TokenKind::Trait,
    TokenKind::Impl,
    TokenKind::Import,
    TokenKind::Pub,
    TokenKind::Eof,
];

/// Check if a token kind is in a set.
pub fn is_in_set(kind: &TokenKind, set: &[TokenKind]) -> bool {
    set.iter().any(|k| std::mem::discriminant(k) == std::mem::discriminant(kind))
}

/// Check if a token starts a statement.
pub fn is_stmt_start(kind: &TokenKind) -> bool {
    is_in_set(kind, STMT_STARTS)
}

/// Check if a token ends a statement.
pub fn is_stmt_end(kind: &TokenKind) -> bool {
    is_in_set(kind, STMT_ENDS)
}

/// Check if a token is a synchronization point.
pub fn is_sync_token(kind: &TokenKind) -> bool {
    is_in_set(kind, SYNC_TOKENS)
}

/// Recovery mode for the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    /// Recover to the next statement.
    Statement,
    /// Recover to the next expression terminator.
    Expression,
    /// Recover to a closing delimiter.
    Delimiter(DelimiterKind),
    /// No recovery - stop at first error.
    None,
}

/// Delimiter kinds for recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterKind {
    Paren,   // )
    Bracket, // ]
    Brace,   // }
}

impl DelimiterKind {
    pub fn closing_token(&self) -> TokenKind {
        match self {
            DelimiterKind::Paren => TokenKind::RParen,
            DelimiterKind::Bracket => TokenKind::RBracket,
            DelimiterKind::Brace => TokenKind::RBrace,
        }
    }
    
    pub fn opening_token(&self) -> TokenKind {
        match self {
            DelimiterKind::Paren => TokenKind::LParen,
            DelimiterKind::Bracket => TokenKind::LBracket,
            DelimiterKind::Brace => TokenKind::LBrace,
        }
    }
}

/// Tracks nested delimiters for balanced recovery.
#[derive(Debug, Default)]
pub struct DelimiterStack {
    stack: Vec<DelimiterKind>,
}

impl DelimiterStack {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn push(&mut self, kind: DelimiterKind) {
        self.stack.push(kind);
    }
    
    pub fn pop(&mut self) -> Option<DelimiterKind> {
        self.stack.pop()
    }
    
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
    
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
    
    /// Check if we're inside a specific delimiter.
    pub fn inside(&self, kind: DelimiterKind) -> bool {
        self.stack.contains(&kind)
    }
    
    /// Update stack based on current token.
    pub fn update(&mut self, token: &TokenKind) {
        match token {
            TokenKind::LParen => self.push(DelimiterKind::Paren),
            TokenKind::LBracket => self.push(DelimiterKind::Bracket),
            TokenKind::LBrace | TokenKind::HashLBrace => self.push(DelimiterKind::Brace),
            TokenKind::RParen => {
                if self.stack.last() == Some(&DelimiterKind::Paren) {
                    self.pop();
                }
            }
            TokenKind::RBracket => {
                if self.stack.last() == Some(&DelimiterKind::Bracket) {
                    self.pop();
                }
            }
            TokenKind::RBrace => {
                if self.stack.last() == Some(&DelimiterKind::Brace) {
                    self.pop();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_stmt_start() {
        assert!(is_stmt_start(&TokenKind::Let));
        assert!(is_stmt_start(&TokenKind::Fn));
        assert!(!is_stmt_start(&TokenKind::Plus));
    }
    
    #[test]
    fn test_delimiter_stack() {
        let mut stack = DelimiterStack::new();
        assert!(stack.is_empty());
        
        stack.update(&TokenKind::LParen);
        assert_eq!(stack.depth(), 1);
        assert!(stack.inside(DelimiterKind::Paren));
        
        stack.update(&TokenKind::LBrace);
        assert_eq!(stack.depth(), 2);
        
        stack.update(&TokenKind::RBrace);
        assert_eq!(stack.depth(), 1);
        
        stack.update(&TokenKind::RParen);
        assert!(stack.is_empty());
    }
}
