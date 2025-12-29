//! The Neve lexer.
//! Neve 词法分析器。

use crate::token::{Token, TokenKind};
use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};

/// Mode for lexer state machine.
/// 词法分析器状态机的模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerMode {
    /// Normal mode - 正常模式
    Normal,
    /// Inside interpolated string, expecting string parts or `{`
    /// 在插值字符串内部，等待字符串片段或 `{`
    InInterpolatedString,
    /// Inside interpolation `{...}`, counting brace depth
    /// 在插值表达式 `{...}` 内部，计算花括号深度
    InInterpolation { depth: u32 },
}

/// The Neve lexer.
/// Neve 词法分析器。
///
/// Converts source code into a sequence of tokens.
/// 将源代码转换为 token 序列。
pub struct Lexer<'src> {
    /// Character iterator with position info
    /// 带位置信息的字符迭代器
    chars: std::iter::Peekable<std::str::CharIndices<'src>>,
    /// Current position in source
    /// 当前在源码中的位置
    pos: usize,
    /// Collected diagnostics (errors/warnings)
    /// 收集的诊断信息（错误/警告）
    diagnostics: Vec<Diagnostic>,
    /// Stack of lexer modes for handling nested contexts
    /// 词法分析器模式栈，用于处理嵌套上下文
    mode_stack: Vec<LexerMode>,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source code.
    /// 为给定的源代码创建新的词法分析器。
    pub fn new(source: &'src str) -> Self {
        Self {
            chars: source.char_indices().peekable(),
            pos: 0,
            diagnostics: Vec::new(),
            mode_stack: vec![LexerMode::Normal],
        }
    }

    /// Get the current lexer mode.
    /// 获取当前词法分析器模式。
    fn current_mode(&self) -> LexerMode {
        *self.mode_stack.last().unwrap_or(&LexerMode::Normal)
    }

    /// Push a new mode onto the stack.
    /// 将新模式压入栈中。
    fn push_mode(&mut self, mode: LexerMode) {
        self.mode_stack.push(mode);
    }

    /// Pop the current mode from the stack.
    /// 从栈中弹出当前模式。
    fn pop_mode(&mut self) {
        if self.mode_stack.len() > 1 {
            self.mode_stack.pop();
        }
    }

    /// Tokenize the entire source and return tokens and diagnostics.
    /// 对整个源代码进行词法分析，返回 token 列表和诊断信息。
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

    /// Get the next token based on current mode.
    /// 根据当前模式获取下一个 token。
    fn next_token(&mut self) -> Token {
        match self.current_mode() {
            LexerMode::InInterpolatedString => return self.interpolated_string_part(),
            LexerMode::InInterpolation { depth } => {
                // Handle brace counting for nested braces inside interpolation
                // 处理插值内部嵌套花括号的计数
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
                            // 插值结束，返回字符串模式
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

    /// Get the next token in normal mode.
    /// 在正常模式下获取下一个 token。
    fn next_token_normal(&mut self) -> Token {
        // Skip whitespace - 跳过空白字符
        self.skip_whitespace();

        let start = self.pos;

        // Check for end of file - 检查是否到达文件末尾
        let Some((_pos, ch)) = self.advance() else {
            return Token::new(TokenKind::Eof, Span::from_usize(start, start));
        };

        let kind = match ch {
            // Single character tokens - 单字符 token
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

            // Record literal #{ - 记录字面量 #{
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
            // 点、双点、或以 ./ 或 ../ 开头的路径
            '.' => {
                if self.peek_char() == Some('.') {
                    self.advance();
                    if self.peek_char() == Some('/') {
                        // Path starting with ../ - 以 ../ 开头的路径
                        self.scan_path(start, "..")
                    } else {
                        TokenKind::DotDot
                    }
                } else if self.peek_char() == Some('/') {
                    // Path starting with ./ - 以 ./ 开头的路径
                    self.scan_path(start, ".")
                } else {
                    TokenKind::Dot
                }
            }

            // Colon - 冒号
            ':' => TokenKind::Colon,

            // Plus or PlusPlus - 加号或双加号
            '+' => {
                if self.peek_char() == Some('+') {
                    self.advance();
                    TokenKind::PlusPlus
                } else {
                    TokenKind::Plus
                }
            }

            // Minus, Arrow, or Comment - 减号、箭头或注释
            '-' => {
                if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek_char() == Some('-') {
                    // Could be line comment (-- ...) or block comment (-- -- ... -- --)
                    // 可能是行注释 (-- ...) 或块注释 (-- -- ... -- --)
                    self.advance(); // consume second -
                    if self.peek_char() == Some(' ')
                        && self.peek_nth(1) == Some('-')
                        && self.peek_nth(2) == Some('-')
                    {
                        // Block comment: -- -- ... -- --
                        // 块注释：-- -- ... -- --
                        self.advance(); // skip space
                        self.advance(); // skip -
                        self.advance(); // skip -
                        self.skip_block_comment();
                    } else {
                        // Line comment: -- to end of line
                        // 行注释：-- 到行尾
                        self.skip_line_comment();
                    }
                    return self.next_token();
                } else {
                    TokenKind::Minus
                }
            }

            // Star - 星号（乘号）
            '*' => TokenKind::Star,

            // Slash, SlashSlash, or absolute path
            // 斜杠、双斜杠、或绝对路径
            '/' => {
                if self.peek_char() == Some('/') {
                    self.advance();
                    TokenKind::SlashSlash
                } else if Self::is_path_start_char(self.peek_char()) {
                    // Absolute path starting with /
                    // 以 / 开头的绝对路径
                    self.scan_absolute_path()
                } else {
                    TokenKind::Slash
                }
            }

            // Equals - 等号
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

            // Bang (not) - 感叹号（逻辑非）
            '!' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }

            // Less than - 小于号
            '<' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }

            // Greater than - 大于号
            '>' => {
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }

            // Ampersand (logical and) - & 符号（逻辑与）
            '&' => {
                if self.peek_char() == Some('&') {
                    self.advance();
                    TokenKind::AndAnd
                } else {
                    self.error_unexpected_char(ch, start);
                    TokenKind::Error
                }
            }

            // Pipe - 管道符号
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

            // Question mark - 问号
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

            // String literal - 字符串字面量
            '"' => self.string_literal(),

            // Char literal - 字符字面量
            '\'' => self.char_literal(),

            // Backtick string (interpolated) - 反引号字符串（插值）
            '`' => self.interpolated_string(),

            // Numbers - 数字
            '0'..='9' => self.number(ch),

            // Identifiers and keywords - 标识符和关键字
            'a'..='z' | 'A'..='Z' | '_' => self.identifier(ch),

            _ => {
                self.error_unexpected_char(ch, start);
                TokenKind::Error
            }
        };

        Token::new(kind, Span::from_usize(start, self.pos))
    }

    /// Advance to the next character.
    /// 前进到下一个字符。
    fn advance(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((pos, ch)) = result {
            self.pos = pos + ch.len_utf8();
        }
        result
    }

    /// Peek at the next character without consuming it.
    /// 查看下一个字符但不消耗它。
    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    /// Peek at the nth character ahead.
    /// 查看前方第 n 个字符。
    fn peek_nth(&self, n: usize) -> Option<char> {
        self.chars.clone().nth(n).map(|(_, ch)| ch)
    }

    /// Skip whitespace characters.
    /// 跳过空白字符。
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skip a line comment (-- to end of line).
    /// 跳过行注释（-- 到行尾）。
    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// Skip a block comment (-- -- ... -- --).
    /// 跳过块注释（-- -- ... -- --）。
    fn skip_block_comment(&mut self) {
        loop {
            match self.advance() {
                Some((_, '-')) => {
                    if self.peek_char() == Some('-') {
                        self.advance();
                        // Check for closing: -- -- (space then --)
                        // 检查结束标记：-- --（空格后跟 --）
                        if self.peek_char() == Some(' ')
                            && self.peek_nth(1) == Some('-')
                            && self.peek_nth(2) == Some('-')
                        {
                            self.advance(); // skip space
                            self.advance(); // skip -
                            self.advance(); // skip -
                            break;
                        }
                    }
                }
                None => {
                    // Unterminated comment - 未终止的注释
                    let span = Span::from_usize(self.pos, self.pos);
                    self.diagnostics.push(
                        Diagnostic::error(
                            DiagnosticKind::Lexer,
                            span,
                            "unterminated block comment",
                        )
                        .with_code(ErrorCode::UnterminatedComment),
                    );
                    break;
                }
                _ => {}
            }
        }
    }

    /// Parse a string literal (double-quoted).
    /// 解析字符串字面量（双引号包围）。
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

    /// Parse a character literal (single-quoted).
    /// 解析字符字面量（单引号包围）。
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
                    Diagnostic::error(
                        DiagnosticKind::Lexer,
                        span,
                        "unterminated character literal",
                    )
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

    /// Parse an escape character sequence.
    /// 解析转义字符序列。
    fn escape_char(&mut self) -> Option<char> {
        match self.advance() {
            Some((_, 'n')) => Some('\n'),   // newline - 换行
            Some((_, 'r')) => Some('\r'),   // carriage return - 回车
            Some((_, 't')) => Some('\t'),   // tab - 制表符
            Some((_, '0')) => Some('\0'),   // null - 空字符
            Some((_, '\\')) => Some('\\'),  // backslash - 反斜杠
            Some((_, '"')) => Some('"'),    // double quote - 双引号
            Some((_, '\'')) => Some('\''),  // single quote - 单引号
            Some((_, '{')) => Some('{'),    // left brace - 左花括号
            Some((_, '}')) => Some('}'),    // right brace - 右花括号
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

    /// Start parsing an interpolated string (backtick).
    /// 开始解析插值字符串（反引号）。
    fn interpolated_string(&mut self) -> TokenKind {
        // Start of interpolated string - enter interpolated string mode
        // 插值字符串开始 - 进入插值字符串模式
        self.push_mode(LexerMode::InInterpolatedString);
        TokenKind::InterpolatedStart
    }

    /// Parse a part of an interpolated string.
    /// 解析插值字符串的一部分。
    fn interpolated_string_part(&mut self) -> Token {
        let start = self.pos;
        let mut value = String::new();

        loop {
            match self.peek_char() {
                Some('`') => {
                    // End of interpolated string - 插值字符串结束
                    if !value.is_empty() {
                        // Emit accumulated string part first
                        // 先输出累积的字符串部分
                        return Token::new(
                            TokenKind::InterpolatedPart(value),
                            Span::from_usize(start, self.pos),
                        );
                    }
                    self.advance();
                    self.pop_mode();
                    return Token::new(
                        TokenKind::InterpolatedEnd,
                        Span::from_usize(start, self.pos),
                    );
                }
                Some('{') => {
                    // Start of interpolation - 插值表达式开始
                    if !value.is_empty() {
                        // Emit accumulated string part first
                        // 先输出累积的字符串部分
                        return Token::new(
                            TokenKind::InterpolatedPart(value),
                            Span::from_usize(start, self.pos),
                        );
                    }
                    self.advance();
                    self.pop_mode();
                    self.push_mode(LexerMode::InInterpolation { depth: 0 });
                    return Token::new(
                        TokenKind::InterpolationStart,
                        Span::from_usize(start, self.pos),
                    );
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

    /// Parse a number literal (integer or float).
    /// 解析数字字面量（整数或浮点数）。
    fn number(&mut self, first: char) -> TokenKind {
        let mut value = String::from(first);
        let mut is_float = false;

        // Check for hex, octal, binary - 检查十六进制、八进制、二进制
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

        // Integer part - 整数部分
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

        // Decimal part - 小数部分
        if self.peek_char() == Some('.') {
            // Look ahead to check it's not .. or a method call
            // 向前查看，确保不是 .. 或方法调用
            let mut chars = self.chars.clone();
            chars.next(); // skip .
            if let Some((_, ch)) = chars.next()
                && ch.is_ascii_digit()
            {
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

        // Exponent - 指数部分
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

    /// Parse a hexadecimal number (0x...).
    /// 解析十六进制数字（0x...）。
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

    /// Parse an octal number (0o...).
    /// 解析八进制数字（0o...）。
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

    /// Parse a binary number (0b...).
    /// 解析二进制数字（0b...）。
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

    /// Parse an identifier or keyword.
    /// 解析标识符或关键字。
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

        // Check for keywords - 检查是否为关键字
        TokenKind::keyword_from_str(&value).unwrap_or(TokenKind::Ident(value))
    }

    /// Report an unexpected character error.
    /// 报告意外字符错误。
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

    /// Check if a character can start a path component after /.
    /// 检查字符是否可以作为 / 后面路径组件的开始。
    fn is_path_start_char(ch: Option<char>) -> bool {
        match ch {
            Some(c) => c.is_alphanumeric() || c == '_' || c == '-' || c == '.',
            None => false,
        }
    }

    /// Check if a character is valid in a path.
    /// 检查字符是否在路径中有效。
    fn is_path_char(ch: char) -> bool {
        ch.is_alphanumeric() || matches!(ch, '/' | '_' | '-' | '.' | '+' | '~')
    }

    /// Scan a path literal starting with prefix (., ..).
    /// 扫描以前缀（.、..）开头的路径字面量。
    fn scan_path(&mut self, _start: usize, prefix: &str) -> TokenKind {
        let mut path = String::from(prefix);

        // Consume the initial / after prefix
        // 消耗前缀后的初始 /
        if let Some((_, '/')) = self.advance() {
            path.push('/');
        }

        // Consume path characters - 消耗路径字符
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

    /// Scan an absolute path starting with / (already consumed).
    /// 扫描以 / 开头的绝对路径（/ 已消耗）。
    fn scan_absolute_path(&mut self) -> TokenKind {
        let mut path = String::from("/");

        // Consume path characters - 消耗路径字符
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
