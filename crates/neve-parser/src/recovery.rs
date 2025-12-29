//! Error recovery strategies for the parser.
//! 解析器的错误恢复策略。
//!
//! This module provides utilities for recovering from parse errors
//! so the parser can continue and report multiple errors.
//! 本模块提供从解析错误中恢复的工具，使解析器能够继续运行并报告多个错误。

use neve_lexer::TokenKind;

/// Tokens that typically start a new statement/item.
/// 通常用于开始新语句/项的 token。
pub const STMT_STARTS: &[TokenKind] = &[
    TokenKind::Let,    // let - 变量绑定
    TokenKind::Fn,     // fn - 函数定义
    TokenKind::Type,   // type - 类型别名
    TokenKind::Struct, // struct - 结构体定义
    TokenKind::Enum,   // enum - 枚举定义
    TokenKind::Trait,  // trait - 特征定义
    TokenKind::Impl,   // impl - 实现块
    TokenKind::Import, // import - 导入语句
    TokenKind::Pub,    // pub - 公开可见性
];

/// Tokens that typically end a statement.
/// 通常用于结束语句的 token。
pub const STMT_ENDS: &[TokenKind] = &[
    TokenKind::Semicolon, // ; - 分号
    TokenKind::RBrace,    // } - 右大括号
];

/// Tokens that are synchronization points.
/// 同步点 token，用于错误恢复时定位。
///
/// These tokens mark boundaries where the parser can safely resume
/// after encountering an error.
/// 这些 token 标记了解析器在遇到错误后可以安全恢复的边界。
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
/// 检查某个 token 类型是否在指定集合中。
pub fn is_in_set(kind: &TokenKind, set: &[TokenKind]) -> bool {
    set.iter()
        .any(|k| std::mem::discriminant(k) == std::mem::discriminant(kind))
}

/// Check if a token starts a statement.
/// 检查某个 token 是否是语句开始符。
pub fn is_stmt_start(kind: &TokenKind) -> bool {
    is_in_set(kind, STMT_STARTS)
}

/// Check if a token ends a statement.
/// 检查某个 token 是否是语句结束符。
pub fn is_stmt_end(kind: &TokenKind) -> bool {
    is_in_set(kind, STMT_ENDS)
}

/// Check if a token is a synchronization point.
/// 检查某个 token 是否是同步点。
pub fn is_sync_token(kind: &TokenKind) -> bool {
    is_in_set(kind, SYNC_TOKENS)
}

/// Recovery mode for the parser.
/// 解析器的恢复模式。
///
/// Determines how the parser should recover when encountering an error.
/// 决定解析器在遇到错误时应如何恢复。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    /// Recover to the next statement.
    /// 恢复到下一条语句。
    Statement,
    /// Recover to the next expression terminator.
    /// 恢复到下一个表达式终结符。
    Expression,
    /// Recover to a closing delimiter.
    /// 恢复到闭合定界符。
    Delimiter(DelimiterKind),
    /// No recovery - stop at first error.
    /// 不恢复 - 在第一个错误处停止。
    None,
}

/// Delimiter kinds for recovery.
/// 用于恢复的定界符类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterKind {
    /// Parenthesis: ()
    /// 圆括号
    Paren,
    /// Bracket: []
    /// 方括号
    Bracket,
    /// Brace: {}
    /// 大括号
    Brace,
}

impl DelimiterKind {
    /// Get the closing token for this delimiter kind.
    /// 获取此定界符类型的闭合 token。
    pub fn closing_token(&self) -> TokenKind {
        match self {
            DelimiterKind::Paren => TokenKind::RParen,
            DelimiterKind::Bracket => TokenKind::RBracket,
            DelimiterKind::Brace => TokenKind::RBrace,
        }
    }

    /// Get the opening token for this delimiter kind.
    /// 获取此定界符类型的开放 token。
    pub fn opening_token(&self) -> TokenKind {
        match self {
            DelimiterKind::Paren => TokenKind::LParen,
            DelimiterKind::Bracket => TokenKind::LBracket,
            DelimiterKind::Brace => TokenKind::LBrace,
        }
    }
}

/// Tracks nested delimiters for balanced recovery.
/// 跟踪嵌套定界符以实现平衡恢复。
///
/// This stack maintains the current nesting level of delimiters,
/// which is essential for proper error recovery when parsing
/// nested structures like function calls or blocks.
/// 此栈维护当前定界符的嵌套层级，这对于解析函数调用或代码块等
/// 嵌套结构时的正确错误恢复至关重要。
#[derive(Debug, Default)]
pub struct DelimiterStack {
    /// The stack of open delimiters.
    /// 开放定界符栈。
    stack: Vec<DelimiterKind>,
}

impl DelimiterStack {
    /// Create a new empty delimiter stack.
    /// 创建一个新的空定界符栈。
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a delimiter onto the stack.
    /// 将定界符压入栈中。
    pub fn push(&mut self, kind: DelimiterKind) {
        self.stack.push(kind);
    }

    /// Pop a delimiter from the stack.
    /// 从栈中弹出定界符。
    pub fn pop(&mut self) -> Option<DelimiterKind> {
        self.stack.pop()
    }

    /// Check if the stack is empty.
    /// 检查栈是否为空。
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get the current nesting depth.
    /// 获取当前嵌套深度。
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Check if we're inside a specific delimiter.
    /// 检查当前是否在特定定界符内部。
    pub fn inside(&self, kind: DelimiterKind) -> bool {
        self.stack.contains(&kind)
    }

    /// Update stack based on current token.
    /// 根据当前 token 更新栈状态。
    ///
    /// This method should be called for each token to maintain
    /// accurate delimiter tracking.
    /// 应为每个 token 调用此方法以保持准确的定界符跟踪。
    pub fn update(&mut self, token: &TokenKind) {
        match token {
            // Opening delimiters - push onto stack
            // 开放定界符 - 压入栈
            TokenKind::LParen => self.push(DelimiterKind::Paren),
            TokenKind::LBracket => self.push(DelimiterKind::Bracket),
            TokenKind::LBrace | TokenKind::HashLBrace => self.push(DelimiterKind::Brace),
            // Closing delimiters - pop from stack if matching
            // 闭合定界符 - 如果匹配则从栈中弹出
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
