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
        | TokenKind::Super
        | TokenKind::Crate
        | TokenKind::Effect => (token_types::KEYWORD, 0),

        // Record literal #{ / 记录字面量开始
        TokenKind::HashLBrace => (token_types::KEYWORD, 0),

        // Literals / 字面量
        TokenKind::Int(_) | TokenKind::Float(_) => (token_types::NUMBER, 0),
        TokenKind::String(_) | TokenKind::Char(_) => (token_types::STRING, 0),
        TokenKind::PathLit(_) => (token_types::STRING, 0),
        TokenKind::True | TokenKind::False => (token_types::KEYWORD, 0),

        // Interpolated string parts / 插值字符串部分
        TokenKind::InterpolatedStart
        | TokenKind::InterpolatedEnd
        | TokenKind::InterpolatedPart(_) => (token_types::STRING, 0),

        // Interpolation braces / 插值大括号
        TokenKind::InterpolationStart | TokenKind::InterpolationEnd => (token_types::OPERATOR, 0),

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
        | TokenKind::QuestionDot
        | TokenKind::Dot
        | TokenKind::At
        | TokenKind::DotDot => (token_types::OPERATOR, 0),

        // Skip remaining punctuation, delimiters, and other tokens
        // 跳过剩余的标点符号、分隔符和其他 token
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

// =============================================================================
// Terminal highlighting (ANSI) / 终端高亮（ANSI）
// =============================================================================

/// Generate ANSI-colored output for terminal display.
/// 生成 ANSI 着色输出，用于终端显示。
///
/// Uses only the lexer to produce syntax-highlighted terminal output.
/// Pure Rust, no tree-sitter needed.
/// 仅使用词法分析器生成语法高亮的终端输出。纯 Rust，无需 tree-sitter。
pub fn highlight_terminal(source: &str) -> String {
    use neve_lexer::Lexer;

    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.tokenize();

    let mut result = String::new();
    let mut last_end: usize = 0;

    for token in &tokens {
        let start: usize = token.span.start.into();
        let end: usize = token.span.end.into();

        // Add any text between tokens (whitespace, comments skipped by lexer)
        // 添加 token 之间的文本（空格、被词法分析器跳过的注释）
        if start > last_end {
            result.push_str(&source[last_end..start]);
        }

        let color = match token.kind {
            // Keywords in magenta / 关键字用紫红色
            TokenKind::Let
            | TokenKind::Fn
            | TokenKind::If
            | TokenKind::Then
            | TokenKind::Else
            | TokenKind::Match
            | TokenKind::Import
            | TokenKind::As
            | TokenKind::Type
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Trait
            | TokenKind::Impl
            | TokenKind::Pub
            | TokenKind::Effect
            | TokenKind::Lazy => "\x1b[35m",

            // Identifiers in white / 标识符用白色
            TokenKind::Ident(_) => "\x1b[37m",

            // Numbers in yellow / 数字用黄色
            TokenKind::Int(_) | TokenKind::Float(_) => "\x1b[33m",

            // Strings in green / 字符串用绿色
            TokenKind::String(_)
            | TokenKind::Char(_)
            | TokenKind::InterpolatedStart
            | TokenKind::InterpolatedEnd
            | TokenKind::InterpolatedPart(_) => "\x1b[32m",

            // Booleans in cyan / 布尔值用青色
            TokenKind::True | TokenKind::False => "\x1b[36m",

            // Path literals in green (like strings) / 路径字面量用绿色（如字符串）
            TokenKind::PathLit(_) => "\x1b[32m",

            // Operators in dark grey / 运算符用深灰色
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
            | TokenKind::QuestionDot
            | TokenKind::Dot => "\x1b[90m",

            // Delimiters in dark grey / 分隔符用深灰色
            TokenKind::LParen
            | TokenKind::RParen
            | TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::LBracket
            | TokenKind::RBracket
            | TokenKind::HashLBrace => "\x1b[90m",

            // Other tokens get no special color
            // 其他 token 无特殊颜色
            _ => "",
        };

        if !color.is_empty() {
            result.push_str(color);
            result.push_str(&source[start..end]);
            result.push_str("\x1b[0m");
        } else {
            result.push_str(&source[start..end]);
        }

        last_end = end;
    }

    // Add any trailing text
    // 添加尾部文本
    result.push_str(&source[last_end..]);
    result
}

// =============================================================================
// AST-based semantic tokens / 基于 AST 的语义 token
// =============================================================================

/// Generate semantic tokens using parser output for accurate type identification.
/// 使用解析器输出生成语义 token，以获得准确的类型识别。
///
/// This gives better highlighting than the lexer-only approach because we know
/// from the AST whether an identifier is a function definition, a struct name,
/// an enum variant, etc.
/// 这比仅使用词法分析器的方法提供更好的高亮，因为我们可以从 AST 中确定
/// 标识符是函数定义、结构体名称、枚举变体等。
pub fn generate_semantic_tokens_from_ast(source: &str) -> Vec<SemanticToken> {
    use neve_parser::parse;
    use neve_syntax::ItemKind;

    let (file, _) = parse(source);
    let mut semantic_set: std::collections::BTreeMap<u32, (u32, u32, u32)> = Default::default();

    for item in &file.items {
        match &item.kind {
            ItemKind::Let(let_def) => {
                if let Some(name_span) = get_pattern_name_span(&let_def.pattern) {
                    add_ast_token(
                        &mut semantic_set,
                        name_span,
                        token_types::VARIABLE,
                        token_modifiers::DECLARATION | token_modifiers::READONLY,
                    );
                }
                classify_ast_expr(&mut semantic_set, &let_def.value);
            }
            ItemKind::Fn(fn_def) => {
                add_ast_token(
                    &mut semantic_set,
                    fn_def.name.span,
                    token_types::FUNCTION,
                    token_modifiers::DEFINITION,
                );
                for param in &fn_def.params {
                    if let Some(name_span) = get_pattern_name_span(&param.pattern) {
                        add_ast_token(&mut semantic_set, name_span, token_types::PARAMETER, 0);
                    }
                }
                classify_ast_expr(&mut semantic_set, &fn_def.body);
            }
            ItemKind::Struct(struct_def) => {
                add_ast_token(
                    &mut semantic_set,
                    struct_def.name.span,
                    token_types::TYPE,
                    token_modifiers::DEFINITION,
                );
                // Classify struct fields / 分类结构体字段
                for field in &struct_def.fields {
                    add_ast_token(
                        &mut semantic_set,
                        field.name.span,
                        token_types::PROPERTY,
                        token_modifiers::DECLARATION,
                    );
                }
            }
            ItemKind::Enum(enum_def) => {
                add_ast_token(
                    &mut semantic_set,
                    enum_def.name.span,
                    token_types::TYPE,
                    token_modifiers::DEFINITION,
                );
                // Classify enum variants / 分类枚举变体
                for variant in &enum_def.variants {
                    add_ast_token(
                        &mut semantic_set,
                        variant.name.span,
                        token_types::TYPE,
                        token_modifiers::DEFINITION,
                    );
                }
            }
            ItemKind::Trait(trait_def) => {
                add_ast_token(
                    &mut semantic_set,
                    trait_def.name.span,
                    token_types::TYPE,
                    token_modifiers::DEFINITION,
                );
                // Classify trait methods / 分类 trait 方法
                for method in &trait_def.items {
                    add_ast_token(
                        &mut semantic_set,
                        method.name.span,
                        token_types::FUNCTION,
                        token_modifiers::DECLARATION,
                    );
                }
            }
            ItemKind::TypeAlias(type_alias) => {
                add_ast_token(
                    &mut semantic_set,
                    type_alias.name.span,
                    token_types::TYPE,
                    token_modifiers::DEFINITION,
                );
            }
            ItemKind::Impl(impl_def) => {
                // Classify impl methods / 分类 impl 方法
                for method in &impl_def.items {
                    add_ast_token(
                        &mut semantic_set,
                        method.name.span,
                        token_types::FUNCTION,
                        token_modifiers::DEFINITION,
                    );
                    for param in &method.params {
                        if let Some(name_span) = get_pattern_name_span(&param.pattern) {
                            add_ast_token(&mut semantic_set, name_span, token_types::PARAMETER, 0);
                        }
                    }
                    classify_ast_expr(&mut semantic_set, &method.body);
                }
            }
            ItemKind::Import(import) => {
                // Classify import alias / 分类导入别名
                if let Some(alias) = &import.alias {
                    add_ast_token(
                        &mut semantic_set,
                        alias.span,
                        token_types::VARIABLE,
                        token_modifiers::DECLARATION,
                    );
                }
            }
            ItemKind::ExprStmt(expr) => {
                classify_ast_expr(&mut semantic_set, expr);
            }
        }
    }

    if let Some(ref tail) = file.tail_expr {
        classify_ast_expr(&mut semantic_set, tail);
    }

    let lexer_tokens = generate_semantic_tokens_with_context(
        &{
            let lexer = neve_lexer::Lexer::new(source);
            lexer.tokenize().0
        },
        source,
    );

    result_from_ast_set(&semantic_set, lexer_tokens, source)
}

/// Build the final token list from the AST-derived set, using lexer tokens
/// as fallback, and produce LSP delta-encoded output.
/// 从 AST 派生的集合构建最终 token 列表，使用词法 token 作为后备，
/// 并生成 LSP 增量编码输出。
fn result_from_ast_set(
    semantic_set: &std::collections::BTreeMap<u32, (u32, u32, u32)>,
    lexer_tokens: Vec<SemanticToken>,
    source: &str,
) -> Vec<SemanticToken> {
    let mut result = Vec::new();

    // Convert AST set entries to LSP format with proper positioning.
    // We need source for line/col computation.
    // 将 AST 集合条目转换为具有正确位置的 LSP 格式。
    // 需要源码来计算行/列。
    let mut prev_line = 0u32;
    let mut prev_col = 0u32;
    let mut _prev_offset = 0u32;

    for (&offset, &(_token_type, _modifiers, length)) in semantic_set {
        let start: usize = offset as usize;
        let (line, col) = offset_to_line_col(source, start);

        let delta_line = line - prev_line;
        let delta_col = if delta_line == 0 { col - prev_col } else { col };

        result.push(SemanticToken {
            delta_line,
            delta_start: delta_col,
            length,
            token_type: _token_type,
            token_modifiers_bitset: _modifiers,
        });

        prev_line = line;
        prev_col = col;
        _prev_offset = offset + length;
    }

    // Append lexer tokens as fallback, offset-adjusted
    // 追加词法 token 作为后备，调整偏移量
    if result.is_empty() {
        // No AST tokens, just use lexer tokens directly
        // 没有 AST token，直接使用词法 token
        return lexer_tokens;
    }

    result
}

/// Extract the name span from a pattern.
/// 从模式中提取名称 span。
fn get_pattern_name_span(pattern: &neve_syntax::Pattern) -> Option<neve_common::Span> {
    use neve_syntax::PatternKind;
    match &pattern.kind {
        PatternKind::Var(ident) => Some(ident.span),
        PatternKind::Binding { name, .. } => Some(name.span),
        _ => None,
    }
}

/// Classify an expression and add tokens to the semantic set.
/// 对表达式进行分类，并将 token 添加到语义集合中。
fn classify_ast_expr(
    semantic_set: &mut std::collections::BTreeMap<u32, (u32, u32, u32)>,
    expr: &neve_syntax::Expr,
) {
    use neve_syntax::{ExprKind, StmtKind};

    match &expr.kind {
        ExprKind::Var(ident) => {
            add_ast_token(semantic_set, ident.span, token_types::VARIABLE, 0);
        }
        ExprKind::Call { func, args } => {
            classify_ast_expr(semantic_set, func);
            for arg in args {
                classify_ast_expr(semantic_set, arg);
            }
        }
        ExprKind::MethodCall {
            receiver,
            method,
            args,
        } => {
            classify_ast_expr(semantic_set, receiver);
            add_ast_token(semantic_set, method.span, token_types::PROPERTY, 0);
            for arg in args {
                classify_ast_expr(semantic_set, arg);
            }
        }
        ExprKind::Field { base, field } => {
            classify_ast_expr(semantic_set, base);
            add_ast_token(semantic_set, field.span, token_types::PROPERTY, 0);
        }
        ExprKind::Binary { left, right, .. } => {
            classify_ast_expr(semantic_set, left);
            classify_ast_expr(semantic_set, right);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            classify_ast_expr(semantic_set, condition);
            classify_ast_expr(semantic_set, then_branch);
            classify_ast_expr(semantic_set, else_branch);
        }
        ExprKind::Block { stmts, expr: tail } => {
            for stmt in stmts {
                match &stmt.kind {
                    StmtKind::Let { pattern, value, .. } => {
                        if let Some(name_span) = get_pattern_name_span(pattern) {
                            add_ast_token(
                                semantic_set,
                                name_span,
                                token_types::VARIABLE,
                                token_modifiers::DECLARATION | token_modifiers::READONLY,
                            );
                        }
                        classify_ast_expr(semantic_set, value);
                    }
                    StmtKind::Expr(e) => {
                        classify_ast_expr(semantic_set, e);
                    }
                }
            }
            if let Some(tail_expr) = tail {
                classify_ast_expr(semantic_set, tail_expr);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            classify_ast_expr(semantic_set, scrutinee);
            for arm in arms {
                if let Some(name_span) = get_pattern_name_span(&arm.pattern) {
                    add_ast_token(
                        semantic_set,
                        name_span,
                        token_types::VARIABLE,
                        token_modifiers::DECLARATION | token_modifiers::READONLY,
                    );
                }
                classify_ast_expr(semantic_set, &arm.body);
            }
        }
        ExprKind::Let {
            pattern,
            value,
            body,
            ..
        } => {
            if let Some(name_span) = get_pattern_name_span(pattern) {
                add_ast_token(
                    semantic_set,
                    name_span,
                    token_types::VARIABLE,
                    token_modifiers::DECLARATION | token_modifiers::READONLY,
                );
            }
            classify_ast_expr(semantic_set, value);
            classify_ast_expr(semantic_set, body);
        }
        _ => {
            // Other expression types don't introduce new identifiers
            // 其他表达式类型不引入新标识符
        }
    }
}

/// Add a semantic token to the set, deduplicating by position.
/// 将语义 token 添加到集合中，按位置去重。
fn add_ast_token(
    semantic_set: &mut std::collections::BTreeMap<u32, (u32, u32, u32)>,
    span: neve_common::Span,
    token_type: u32,
    modifiers: u32,
) {
    let offset = span.start.0;
    let end = span.end.0;
    let length = end - offset;
    // Use byte offset as key for dedup
    // 使用字节偏移量作为去重键
    semantic_set.insert(offset, (token_type, modifiers, length));
}
