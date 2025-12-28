//! The Neve lexer.

use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};
use crate::token::{Token, TokenKind};

/// Mode for lexer state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerMode {
    /// Normal mode
    Normal,
    /// Inside interpolated string, expecting string parts or `{`
    InInterpolatedString,
    /// Inside interpolation `{...}`, counting brace depth
    InInterpolation { depth: u32 },
}

/// The Neve lexer.
pub struct Lexer<'src> {
    chars: std::iter::Peekable<std::str::CharIndices<'src>>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    mode_stack: Vec<LexerMode>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            chars: source.char_indices().peekable(),
            pos: 0,
            diagnostics: Vec::new(),
            mode_stack: vec![LexerMode::Normal],
        }
    }

    fn current_mode(&self) -> LexerMode {
        *self.mode_stack.last().unwrap_or(&LexerMode::Normal)
    }

    fn push_mode(&mut self, mode: LexerMode) {
        self.mode_stack.push(mode);
    }

    fn pop_mode(&mut self) {
        if self.mode_stack.len() > 1 {
            self.mode_stack.pop();
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
        match self.current_mode() {
            LexerMode::InInterpolatedString => return self.interpolated_string_part(),
            LexerMode::InInterpolation { depth } => {
                // Handle brace counting for nested braces inside interpolation
                let token = self.next_token_normal();
                match token.kind {
                    TokenKind::LBrace | TokenKind::HashLBrace => {
                        let new_depth = depth + 1;
                        self.mode_stack.pop();
                        self.push_mode(LexerMode::InInterpolation { depth: new_depth });
                    }
                    TokenKind::RBrace => {
                        if depth == 0 {
                            // End of interpolation, return to string mode
                            self.pop_mode();
                            self.push_mode(LexerMode::InInterpolatedString);
                            return Token::new(TokenKind::InterpolationEnd, token.span);
                        } else {
                            let new_depth = depth - 1;
                            self.mode_stack.pop();
                            self.push_mode(LexerMode::InInterpolation { depth: new_depth });
                        }
                    }
                    _ => {}
                }
                return token;
            }
            LexerMode::Normal => {}
        }

        self.next_token_normal()
    }

    fn next_token_normal(&mut self) -> Token {
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

            // Dot, DotDot, or path starting with ./ or ../
            '.' => {
                if self.peek_char() == Some('.') {
                    self.advance();
                    if self.peek_char() == Some('/') {
                        // Path starting with ../
                        self.scan_path(start, "..")
                    } else {
                        TokenKind::DotDot
                    }
                } else if self.peek_char() == Some('/') {
                    // Path starting with ./
                    self.scan_path(start, ".")
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

            // Slash, SlashSlash, or absolute path
            '/' => {
                if self.peek_char() == Some('/') {
                    self.advance();
                    TokenKind::SlashSlash
                } else if Self::is_path_start_char(self.peek_char()) {
                    // Absolute path starting with /
                    self.scan_absolute_path()
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
        // Start of interpolated string - enter interpolated string mode
        self.push_mode(LexerMode::InInterpolatedString);
        TokenKind::InterpolatedStart
    }

    fn interpolated_string_part(&mut self) -> Token {
        let start = self.pos;
        let mut value = String::new();

        loop {
            match self.peek_char() {
                Some('`') => {
                    // End of interpolated string
                    if !value.is_empty() {
                        // Emit accumulated string part first
                        return Token::new(
                            TokenKind::InterpolatedPart(value),
                            Span::from_usize(start, self.pos),
                        );
                    }
                    self.advance();
                    self.pop_mode();
                    return Token::new(TokenKind::InterpolatedEnd, Span::from_usize(start, self.pos));
                }
                Some('{') => {
                    // Start of interpolation
                    if !value.is_empty() {
                        // Emit accumulated string part first
                        return Token::new(
                            TokenKind::InterpolatedPart(value),
                            Span::from_usize(start, self.pos),
                        );
                    }
                    self.advance();
                    self.pop_mode();
                    self.push_mode(LexerMode::InInterpolation { depth: 0 });
                    return Token::new(TokenKind::InterpolationStart, Span::from_usize(start, self.pos));
                }
                Some('\\') => {
                    self.advance();
                    if let Some(escaped) = self.escape_char() {
                        value.push(escaped);
                    }
                }
                Some(ch) => {
                    self.advance();
                    value.push(ch);
                }
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
                    self.pop_mode();
                    return Token::new(TokenKind::Error, span);
                }
            }
        }
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

    /// Check if a character can start a path component after /
    fn is_path_start_char(ch: Option<char>) -> bool {
        match ch {
            Some(c) => c.is_alphanumeric() || c == '_' || c == '-' || c == '.',
            None => false,
        }
    }

    /// Check if a character is valid in a path
    fn is_path_char(ch: char) -> bool {
        ch.is_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.' | '+' | '~')
    }

    /// Scan a path literal starting with prefix (., ..)
    fn scan_path(&mut self, _start: usize, prefix: &str) -> TokenKind {
        let mut path = String::from(prefix);
        
        // Consume the initial / after prefix
        if let Some((_, '/')) = self.advance() {
            path.push('/');
        }
        
        // Consume path characters
        while let Some(ch) = self.peek_char() {
            if Self::is_path_char(ch) {
                path.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        TokenKind::PathLit(path)
    }

    /// Scan an absolute path starting with / (already consumed)
    fn scan_absolute_path(&mut self) -> TokenKind {
        let mut path = String::from("/");
        
        // Consume path characters
        while let Some(ch) = self.peek_char() {
            if Self::is_path_char(ch) {
                path.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        TokenKind::PathLit(path)
    }
}
