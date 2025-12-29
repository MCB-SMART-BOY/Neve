//! Token definitions for Neve.
//! Neve 的 Token 定义。

use neve_common::Span;

/// A token with its kind and span.
/// 带有类型和位置信息的 Token。
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind of token - Token 的类型
    pub kind: TokenKind,
    /// The source location - 源码位置
    pub span: Span,
}

impl Token {
    /// Create a new token.
    /// 创建新的 Token。
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kind of a token.
/// Token 的类型。
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ===== Literals 字面量 =====
    
    /// Integer literal - 整数字面量
    Int(i64),
    /// Float literal - 浮点数字面量
    Float(f64),
    /// String literal - 字符串字面量
    String(String),
    /// Character literal - 字符字面量
    Char(char),
    /// Boolean literal - 布尔字面量
    Bool(bool),
    /// Path literal (e.g., `./foo`, `/bar`, `../baz`)
    /// 路径字面量（如 `./foo`、`/bar`、`../baz`）
    PathLit(String),

    // ===== Interpolated string parts 插值字符串部分 =====
    
    /// Start of interpolated string `` ` ``
    /// 插值字符串开始 `` ` ``
    InterpolatedStart,
    /// String part in interpolated string (between { })
    /// 插值字符串中的字符串部分（在 { } 之间）
    InterpolatedPart(String),
    /// End of interpolated string `` ` ``
    /// 插值字符串结束 `` ` ``
    InterpolatedEnd,
    /// Start of interpolation `{`
    /// 插值表达式开始 `{`
    InterpolationStart,
    /// End of interpolation `}`
    /// 插值表达式结束 `}`
    InterpolationEnd,

    // ===== Identifiers 标识符 =====
    
    /// Identifier - 标识符
    Ident(String),

    // ===== Keywords 关键字 =====
    
    Let,       // let - 变量绑定
    Fn,        // fn - 函数
    Type,      // type - 类型别名
    Struct,    // struct - 结构体
    Enum,      // enum - 枚举
    Trait,     // trait - 特征
    Impl,      // impl - 实现
    Pub,       // pub - 公开
    Import,    // import - 导入
    As,        // as - 重命名
    SelfLower, // self - 自身
    Super,     // super - 父模块
    Crate,     // crate - 当前 crate
    If,        // if - 条件
    Then,      // then - 条件成立时
    Else,      // else - 否则
    Match,     // match - 模式匹配
    Lazy,      // lazy - 惰性求值
    True,      // true - 真
    False,     // false - 假

    // ===== Delimiters 分隔符 =====
    
    LParen,     // ( - 左圆括号
    RParen,     // ) - 右圆括号
    LBracket,   // [ - 左方括号
    RBracket,   // ] - 右方括号
    LBrace,     // { - 左花括号
    RBrace,     // } - 右花括号
    HashLBrace, // #{ - 记录开始

    // ===== Operators 操作符 =====
    
    Plus,             // + - 加
    Minus,            // - - 减
    Star,             // * - 乘
    Slash,            // / - 除
    Percent,          // % - 取模
    Caret,            // ^ - 幂
    Eq,               // = - 赋值
    EqEq,             // == - 等于
    BangEq,           // != - 不等于
    Lt,               // < - 小于
    LtEq,             // <= - 小于等于
    Gt,               // > - 大于
    GtEq,             // >= - 大于等于
    AndAnd,           // && - 逻辑与
    OrOr,             // || - 逻辑或
    Bang,             // ! - 逻辑非
    PlusPlus,         // ++ - 列表拼接
    SlashSlash,       // // - 记录合并
    QuestionQuestion, // ?? - 空值合并
    QuestionDot,      // ?. - 安全访问
    Pipe,             // | - 管道/模式或
    PipeGt,           // |> - 管道操作符
    Arrow,            // -> - 箭头
    FatArrow,         // => - 粗箭头
    At,               // @ - 模式绑定
    DotDot,           // .. - 范围/展开
    Question,         // ? - 错误传播

    // ===== Punctuation 标点 =====
    
    Comma,     // , - 逗号
    Colon,     // : - 冒号
    Semicolon, // ; - 分号
    Dot,       // . - 点

    // ===== Special 特殊 =====
    
    /// End of file - 文件结束
    Eof,
    /// Lexer error - 词法错误
    Error,
}

impl TokenKind {
    /// Returns true if this token is a keyword.
    /// 判断是否为关键字。
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
    /// 如果标识符是关键字，返回对应的 TokenKind。
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
