//! The Neve lexer.

use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};
use crate::token::{Token, TokenKind};

/// The Neve lexer.
pub struct Lexer<'src> {
    #[allow(dead_code)]
    source: &'src str,
    chars: std::iter::Peekable<std::str::CharIndices<'src>>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    /// Tokenize the entire source and return tokens and diagnostics.
    pub fn tokenize(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        (tokens, self.diagnostics)
    }

    fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let start = self.pos;

        let Some((_pos, ch)) = self.advance() else {
            return Token::new(TokenKind::Eof, Span::from_usize(start, start));
        };

        let kind = match ch {
            // Single character tokens
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '@' => TokenKind::At,
            '^' => TokenKind::Caret,
            '%' => TokenKind::Percent,

            // Record literal #{
            '#' => {
                if self.peek_char() == Some('{') {
                    self.advance();
                    TokenKind::HashLBrace
                } else {
                    self.error_unexpected_char(ch, start);
                    TokenKind::Error
                }
            }

            // Dot or DotDot
            '.' => {
                if self.peek_char() == Some('.') {
                    self.advance();
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }

            // Colon
            ':' => TokenKind::Colon,

            // Plus or PlusPlus
            '+' => {
                if self.peek_char() == Some('+') {
                    self.advance();
                    TokenKind::PlusPlus
                } else {
                    TokenKind::Plus
                }
            }

            // Minus or Arrow
            '-' => {
                if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek_char() == Some('-') {
                    // Could be line comment (-- ...) or block comment (-- -- ... -- --)
                    self.advance(); // consume second -
                    if self.peek_char() == Some(' ') && self.peek_nth(1) == Some('-') && self.peek_nth(2) == Some('-') {
                        // Block comment: -- -- ... -- --
                        self.advance(); // skip space
                        self.advance(); // skip -
                        self.advance(); // skip -
                        self.skip_block_comment();
                    } else {
                        // Line comment: -- to end of line
                        self.skip_line_comment();
                    }
                    return self.next_token();
                } else {
                    TokenKind::Minus
                }
            }

            // Star
            '*' => TokenKind::Star,

            // Slash or SlashSlash
            '/' => {
                if self.peek_char() == Some('/') {
                    self.advance();
                    TokenKind::SlashSlash
                } else {
                    TokenKind::Slash
                }
            }

            // Equals
            '=' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::EqEq
                } else if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Eq
                }
            }

            // Bang
            '!' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }

            // Less than
            '<' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }

            // Greater than
            '>' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }

            // Ampersand
            '&' => {
                if self.peek_char() == Some('&') {
                    self.advance();
                    TokenKind::AndAnd
                } else {
                    self.error_unexpected_char(ch, start);
                    TokenKind::Error
                }
            }

            // Pipe
            '|' => {
                if self.peek_char() == Some('|') {
                    self.advance();
                    TokenKind::OrOr
                } else if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::PipeGt
                } else {
                    TokenKind::Pipe
                }
            }

            // Question mark
            '?' => {
                if self.peek_char() == Some('?') {
                    self.advance();
                    TokenKind::QuestionQuestion
                } else if self.peek_char() == Some('.') {
                    self.advance();
                    TokenKind::QuestionDot
                } else {
                    TokenKind::Question
                }
            }

            // String literal
            '"' => self.string_literal(),

            // Char literal
            '\'' => self.char_literal(),

            // Backtick string (interpolated)
            '`' => self.interpolated_string(),

            // Numbers
            '0'..='9' => self.number(ch),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.identifier(ch),

            _ => {
                self.error_unexpected_char(ch, start);
                TokenKind::Error
            }
        };

        Token::new(kind, Span::from_usize(start, self.pos))
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((pos, ch)) = result {
            self.pos = pos + ch.len_utf8();
        }
        result
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.chars.clone().nth(n).map(|(_, ch)| ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        // Skip until end of line
        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) {
        // Skip until we find closing -- --
        loop {
            match self.advance() {
                Some((_, '-')) => {
                    if self.peek_char() == Some('-') {
                        self.advance();
                        // Check for closing: -- -- (space then --)
                        if self.peek_char() == Some(' ') && self.peek_nth(1) == Some('-') && self.peek_nth(2) == Some('-') {
                            self.advance(); // skip space
                            self.advance(); // skip -
                            self.advance(); // skip -
                            break;
                        }
                    }
                }
                None => {
                    // Unterminated comment
                    let span = Span::from_usize(self.pos, self.pos);
                    self.diagnostics.push(
                        Diagnostic::error(DiagnosticKind::Lexer, span, "unterminated block comment")
                            .with_code(ErrorCode::UnterminatedComment),
                    );
                    break;
                }
                _ => {}
            }
        }
    }

    fn string_literal(&mut self) -> TokenKind {
        let mut value = String::new();
        let start = self.pos;

        loop {
            match self.advance() {
                Some((_, '"')) => break,
                Some((_, '\\')) => {
                    if let Some(escaped) = self.escape_char() {
                        value.push(escaped);
                    }
                }
                Some((_, ch)) => value.push(ch),
                None => {
                    let span = Span::from_usize(start, self.pos);
                    self.diagnostics.push(
                        Diagnostic::error(DiagnosticKind::Lexer, span, "unterminated string")
                            .with_code(ErrorCode::UnterminatedString),
                    );
                    return TokenKind::Error;
                }
            }
        }

        TokenKind::String(value)
    }

    fn char_literal(&mut self) -> TokenKind {
        let start = self.pos;

        let ch = match self.advance() {
            Some((_, '\\')) => self.escape_char(),
            Some((_, ch)) => Some(ch),
            None => None,
        };

        match self.advance() {
            Some((_, '\'')) => {}
            _ => {
                let span = Span::from_usize(start, self.pos);
                self.diagnostics.push(
                    Diagnostic::error(DiagnosticKind::Lexer, span, "unterminated character literal")
                        .with_code(ErrorCode::UnterminatedString),
                );
                return TokenKind::Error;
            }
        }

        match ch {
            Some(c) => TokenKind::Char(c),
            None => TokenKind::Error,
        }
    }

    fn escape_char(&mut self) -> Option<char> {
        match self.advance() {
            Some((_, 'n')) => Some('\n'),
            Some((_, 'r')) => Some('\r'),
            Some((_, 't')) => Some('\t'),
            Some((_, '0')) => Some('\0'),
            Some((_, '\\')) => Some('\\'),
            Some((_, '"')) => Some('"'),
            Some((_, '\'')) => Some('\''),
            Some((_, '{')) => Some('{'),
            Some((_, '}')) => Some('}'),
            Some((pos, ch)) => {
                let span = Span::from_usize(pos, self.pos);
                self.diagnostics.push(
                    Diagnostic::error(
                        DiagnosticKind::Lexer,
                        span,
                        format!("invalid escape sequence: \\{}", ch),
                    )
                    .with_code(ErrorCode::InvalidEscape),
                );
                None
            }
            None => None,
        }
    }

    fn interpolated_string(&mut self) -> TokenKind {
        // For now, treat as regular string
        // TODO: Proper interpolation parsing
        let mut value = String::new();
        let start = self.pos;

        loop {
            match self.advance() {
                Some((_, '`')) => break,
                Some((_, '\\')) => {
                    if let Some(escaped) = self.escape_char() {
                        value.push(escaped);
                    }
                }
                Some((_, ch)) => value.push(ch),
                None => {
                    let span = Span::from_usize(start, self.pos);
                    self.diagnostics.push(
                        Diagnostic::error(
                            DiagnosticKind::Lexer,
                            span,
                            "unterminated interpolated string",
                        )
                        .with_code(ErrorCode::UnterminatedString),
                    );
                    return TokenKind::Error;
                }
            }
        }

        TokenKind::String(value)
    }

    fn number(&mut self, first: char) -> TokenKind {
        let mut value = String::from(first);
        let mut is_float = false;

        // Check for hex, octal, binary
        if first == '0' {
            match self.peek_char() {
                Some('x' | 'X') => {
                    self.advance();
                    return self.hex_number();
                }
                Some('o' | 'O') => {
                    self.advance();
                    return self.octal_number();
                }
                Some('b' | 'B') => {
                    self.advance();
                    return self.binary_number();
                }
                _ => {}
            }
        }

        // Integer part
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        // Decimal part
        if self.peek_char() == Some('.') {
            // Look ahead to check it's not .. or a method call
            let mut chars = self.chars.clone();
            chars.next(); // skip .
            if let Some((_, ch)) = chars.next()
                && ch.is_ascii_digit() {
                    self.advance(); // consume .
                    value.push('.');
                    is_float = true;

                    while let Some(ch) = self.peek_char() {
                        if ch.is_ascii_digit() || ch == '_' {
                            if ch != '_' {
                                value.push(ch);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
        }

        // Exponent
        if let Some('e' | 'E') = self.peek_char() {
            self.advance();
            value.push('e');
            is_float = true;

            if let Some('+' | '-') = self.peek_char() {
                value.push(self.advance().unwrap().1);
            }

            while let Some(ch) = self.peek_char() {
                if ch.is_ascii_digit() {
                    value.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if is_float {
            match value.parse::<f64>() {
                Ok(f) => TokenKind::Float(f),
                Err(_) => TokenKind::Error,
            }
        } else {
            match value.parse::<i64>() {
                Ok(i) => TokenKind::Int(i),
                Err(_) => TokenKind::Error,
            }
        }
    }

    fn hex_number(&mut self) -> TokenKind {
        let mut value = String::new();

        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_hexdigit() || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        match i64::from_str_radix(&value, 16) {
            Ok(i) => TokenKind::Int(i),
            Err(_) => TokenKind::Error,
        }
    }

    fn octal_number(&mut self) -> TokenKind {
        let mut value = String::new();

        while let Some(ch) = self.peek_char() {
            if ('0'..='7').contains(&ch) || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        match i64::from_str_radix(&value, 8) {
            Ok(i) => TokenKind::Int(i),
            Err(_) => TokenKind::Error,
        }
    }

    fn binary_number(&mut self) -> TokenKind {
        let mut value = String::new();

        while let Some(ch) = self.peek_char() {
            if ch == '0' || ch == '1' || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        match i64::from_str_radix(&value, 2) {
            Ok(i) => TokenKind::Int(i),
            Err(_) => TokenKind::Error,
        }
    }

    fn identifier(&mut self, first: char) -> TokenKind {
        let mut value = String::from(first);

        while let Some(ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for keywords
        TokenKind::keyword_from_str(&value).unwrap_or(TokenKind::Ident(value))
    }

    fn error_unexpected_char(&mut self, ch: char, pos: usize) {
        let span = Span::from_usize(pos, self.pos);
        self.diagnostics.push(
            Diagnostic::error(
                DiagnosticKind::Lexer,
                span,
                format!("unexpected character: '{}'", ch),
            )
            .with_code(ErrorCode::UnexpectedCharacter)
            .with_label(Label::new(span, "unexpected character here")),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<TokenKind> {
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            lex("let fn if then else match"),
            vec![
                TokenKind::Let,
                TokenKind::Fn,
                TokenKind::If,
                TokenKind::Then,
                TokenKind::Else,
                TokenKind::Match,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_numbers() {
        assert_eq!(
            lex("42 3.14 0xFF 0b1010"),
            vec![
                TokenKind::Int(42),
                TokenKind::Float(3.14),
                TokenKind::Int(255),
                TokenKind::Int(10),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_strings() {
        assert_eq!(
            lex(r#""hello" 'a'"#),
            vec![
                TokenKind::String("hello".to_string()),
                TokenKind::Char('a'),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            lex("+ ++ -> |> // ?? ?."),
            vec![
                TokenKind::Plus,
                TokenKind::PlusPlus,
                TokenKind::Arrow,
                TokenKind::PipeGt,
                TokenKind::SlashSlash,
                TokenKind::QuestionQuestion,
                TokenKind::QuestionDot,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_record_literal() {
        assert_eq!(
            lex("#{ x = 1 }"),
            vec![
                TokenKind::HashLBrace,
                TokenKind::Ident("x".to_string()),
                TokenKind::Eq,
                TokenKind::Int(1),
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }
}
