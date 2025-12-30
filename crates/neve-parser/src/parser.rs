//! The Neve parser.
//! Neve 语法解析器。
//!
//! This module implements a recursive descent parser that converts
//! a token stream into an abstract syntax tree (AST).
//! 本模块实现了一个递归下降解析器，将 token 流转换为抽象语法树（AST）。

use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};
use neve_lexer::{Token, TokenKind};
use neve_syntax::*;

use crate::recovery::{
    DelimiterKind, DelimiterStack, RecoveryMode, STMT_ENDS, is_stmt_end, is_stmt_start,
    is_sync_token,
};

/// The Neve parser.
/// Neve 语法解析器。
///
/// Converts a token stream into an abstract syntax tree using
/// recursive descent parsing with operator precedence.
/// 使用递归下降解析和运算符优先级将 token 流转换为抽象语法树。
pub struct Parser {
    /// The input tokens.
    /// 输入的 token 序列。
    tokens: Vec<Token>,
    /// Current position in the token stream.
    /// 在 token 流中的当前位置。
    pos: usize,
    /// Accumulated diagnostics (errors and warnings).
    /// 累积的诊断信息（错误和警告）。
    diagnostics: Vec<Diagnostic>,
    /// Delimiter stack for tracking balanced delimiters.
    /// 用于跟踪平衡定界符的栈。
    delimiter_stack: DelimiterStack,
    /// Current recovery mode.
    /// 当前恢复模式。
    recovery_mode: RecoveryMode,
}

impl Parser {
    /// Create a new parser from a token stream.
    /// 从 token 流创建新的解析器。
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            delimiter_stack: DelimiterStack::new(),
            recovery_mode: RecoveryMode::Statement,
        }
    }

    /// Consume the parser and return accumulated diagnostics.
    /// 消耗解析器并返回累积的诊断信息。
    pub fn diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    // ========== Top-Level Parsing 顶层解析 ==========

    /// Parse a complete source file.
    /// 解析完整的源文件。
    ///
    /// This is the main entry point for parsing. It parses all top-level
    /// items in the source file and returns a SourceFile AST node.
    /// 这是解析的主入口。它解析源文件中的所有顶层项并返回 SourceFile AST 节点。
    pub fn parse_file(&mut self) -> SourceFile {
        let start = self.current_span();
        let mut items = Vec::new();

        while !self.at_end() {
            // Track delimiter state for each token
            // 为每个 token 跟踪定界符状态
            let kind = self.current_kind().clone();
            self.delimiter_stack.update(&kind);

            if let Some(item) = self.parse_item() {
                items.push(item);
            } else {
                // Error recovery: synchronize to next statement boundary
                // 错误恢复：同步到下一个语句边界
                self.synchronize();
            }
        }

        let end = self.current_span();
        SourceFile {
            items,
            span: start.merge(end),
        }
    }

    /// Parse a top-level item.
    /// 解析顶层项。
    ///
    /// Items include: let bindings, functions, type aliases, structs,
    /// enums, traits, impl blocks, and imports.
    /// 项包括：let 绑定、函数、类型别名、结构体、枚举、特征、impl 块和导入。
    fn parse_item(&mut self) -> Option<Item> {
        let start = self.current_span();
        let is_pub = self.eat(TokenKind::Pub);

        let kind = match self.current_kind() {
            TokenKind::Let => {
                self.advance();
                Some(ItemKind::Let(self.parse_let_def(is_pub)))
            }
            TokenKind::Fn => {
                self.advance();
                Some(ItemKind::Fn(self.parse_fn_def(is_pub)))
            }
            TokenKind::Type => {
                self.advance();
                Some(ItemKind::TypeAlias(self.parse_type_alias(is_pub)))
            }
            TokenKind::Struct => {
                self.advance();
                Some(ItemKind::Struct(self.parse_struct_def(is_pub)))
            }
            TokenKind::Enum => {
                self.advance();
                Some(ItemKind::Enum(self.parse_enum_def(is_pub)))
            }
            TokenKind::Trait => {
                self.advance();
                Some(ItemKind::Trait(self.parse_trait_def(is_pub)))
            }
            TokenKind::Impl => {
                self.advance();
                Some(ItemKind::Impl(self.parse_impl_def()))
            }
            TokenKind::Import => {
                self.advance();
                Some(ItemKind::Import(self.parse_import_def(is_pub)))
            }
            _ => {
                if is_pub {
                    self.error("expected item after `pub`");
                }
                None
            }
        };

        kind.map(|k| {
            let end = self.previous_span();
            Item {
                kind: k,
                span: start.merge(end),
            }
        })
    }

    // ========== Item Definitions 项定义 ==========

    /// Parse a let binding definition.
    /// 解析 let 绑定定义。
    ///
    /// Syntax: `let pattern [: type] = expr;`
    /// 语法：`let 模式 [: 类型] = 表达式;`
    fn parse_let_def(&mut self, is_pub: bool) -> LetDef {
        let pattern = self.parse_pattern();
        let ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect(TokenKind::Eq);
        let value = self.parse_expr();
        self.expect(TokenKind::Semicolon);

        LetDef {
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            pattern,
            ty,
            value,
        }
    }

    /// Parse a function definition.
    /// 解析函数定义。
    ///
    /// Syntax: `fn name[<generics>](params) [-> return_type] = body;`
    /// 语法：`fn 名称[<泛型>](参数) [-> 返回类型] = 函数体;`
    fn parse_fn_def(&mut self, is_pub: bool) -> FnDef {
        let name = self.parse_ident();
        let generics = self.parse_generics();
        self.expect(TokenKind::LParen);
        let params = self.parse_params();
        self.expect(TokenKind::RParen);

        let return_type = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };

        self.expect(TokenKind::Eq);
        let body = self.parse_expr();
        self.expect(TokenKind::Semicolon);

        FnDef {
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            name,
            generics,
            params,
            return_type,
            body,
        }
    }

    /// Parse a type alias definition.
    /// 解析类型别名定义。
    ///
    /// Syntax: `type Name[<generics>] = Type;`
    /// 语法：`type 名称[<泛型>] = 类型;`
    fn parse_type_alias(&mut self, is_pub: bool) -> TypeAlias {
        let name = self.parse_ident();
        let generics = self.parse_generics();
        self.expect(TokenKind::Eq);
        let ty = self.parse_type();
        self.expect(TokenKind::Semicolon);

        TypeAlias {
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            name,
            generics,
            ty,
        }
    }

    /// Parse a struct definition.
    /// 解析结构体定义。
    ///
    /// Syntax: `struct Name[<generics>] { fields };`
    /// 语法：`struct 名称[<泛型>] { 字段列表 };`
    fn parse_struct_def(&mut self, is_pub: bool) -> StructDef {
        let name = self.parse_ident();
        let generics = self.parse_generics();
        self.expect(TokenKind::LBrace);
        let fields = self.parse_field_defs();
        self.expect(TokenKind::RBrace);
        self.expect(TokenKind::Semicolon);

        StructDef {
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            name,
            generics,
            fields,
        }
    }

    /// Parse an enum definition.
    /// 解析枚举定义。
    ///
    /// Syntax: `enum Name[<generics>] { variants };`
    /// 语法：`enum 名称[<泛型>] { 变体列表 };`
    fn parse_enum_def(&mut self, is_pub: bool) -> EnumDef {
        let name = self.parse_ident();
        let generics = self.parse_generics();
        self.expect(TokenKind::LBrace);
        let variants = self.parse_variants();
        self.expect(TokenKind::RBrace);
        self.expect(TokenKind::Semicolon);

        EnumDef {
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            name,
            generics,
            variants,
        }
    }

    /// Parse a trait definition.
    /// 解析特征定义。
    ///
    /// Syntax: `trait Name[<generics>] { items };`
    /// 语法：`trait 名称[<泛型>] { 方法和关联类型 };`
    fn parse_trait_def(&mut self, is_pub: bool) -> TraitDef {
        let name = self.parse_ident();
        let generics = self.parse_generics();
        self.expect(TokenKind::LBrace);

        let mut items = Vec::new();
        let mut assoc_types = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.at_end() {
            if self.check(TokenKind::Type) {
                assoc_types.push(self.parse_assoc_type_def());
            } else {
                items.push(self.parse_trait_item());
            }
        }

        self.expect(TokenKind::RBrace);
        self.expect(TokenKind::Semicolon);

        TraitDef {
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
            name,
            generics,
            items,
            assoc_types,
        }
    }

    /// Parse an impl block.
    /// 解析 impl 块。
    ///
    /// Syntax: `impl[<generics>] [Trait for] Type { items };`
    /// 语法：`impl[<泛型>] [特征 for] 类型 { 方法实现 };`
    fn parse_impl_def(&mut self) -> ImplDef {
        let generics = self.parse_generics();
        let first_type = self.parse_type();

        // Check for trait implementation: `impl Trait for Type`
        // 检查是否为特征实现：`impl Trait for Type`
        let (trait_, target) = if self.eat(TokenKind::Ident("for".to_string())) {
            (Some(first_type), self.parse_type())
        } else {
            (None, first_type)
        };

        self.expect(TokenKind::LBrace);

        let mut items = Vec::new();
        let mut assoc_type_impls = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.at_end() {
            if self.check(TokenKind::Type) {
                assoc_type_impls.push(self.parse_assoc_type_impl());
            } else {
                items.push(self.parse_impl_item());
            }
        }

        self.expect(TokenKind::RBrace);
        self.expect(TokenKind::Semicolon);

        ImplDef {
            generics,
            trait_,
            target,
            items,
            assoc_type_impls,
        }
    }

    /// Parse an import definition.
    /// 解析导入定义。
    ///
    /// Syntax: `import [prefix.]path[.(items)] [as alias];`
    /// 语法：`import [前缀.]路径[.(导入项)] [as 别名];`
    fn parse_import_def(&mut self, is_pub: bool) -> ImportDef {
        // Parse optional path prefix (self, super, crate)
        // 解析可选的路径前缀（self、super、crate）
        let prefix = match self.current().kind {
            TokenKind::SelfLower => {
                self.advance();
                self.expect(TokenKind::Dot);
                PathPrefix::Self_
            }
            TokenKind::Super => {
                self.advance();
                self.expect(TokenKind::Dot);
                PathPrefix::Super
            }
            TokenKind::Crate => {
                self.advance();
                self.expect(TokenKind::Dot);
                PathPrefix::Crate
            }
            _ => PathPrefix::Absolute,
        };

        // Parse the path segments
        // 解析路径段
        let mut path = vec![self.parse_ident()];
        while self.eat(TokenKind::Dot) {
            // Check if next token is '(' for import items, not a path segment
            // 检查下一个 token 是否为 '('（导入项），而非路径段
            if matches!(self.current().kind, TokenKind::LParen) {
                break;
            }
            path.push(self.parse_ident());
        }

        // Parse import items: module, all (*), or specific items
        // 解析导入项：模块、全部（*）或特定项
        let items = if self.eat(TokenKind::LParen) {
            if self.eat(TokenKind::Star) {
                self.expect(TokenKind::RParen);
                ImportItems::All
            } else {
                let mut items = vec![self.parse_ident()];
                while self.eat(TokenKind::Comma) {
                    items.push(self.parse_ident());
                }
                self.expect(TokenKind::RParen);
                ImportItems::Items(items)
            }
        } else {
            ImportItems::Module
        };

        // Parse optional alias
        // 解析可选的别名
        let alias = if self.eat(TokenKind::As) {
            Some(self.parse_ident())
        } else {
            None
        };

        self.expect(TokenKind::Semicolon);

        ImportDef {
            prefix,
            path,
            items,
            alias,
            visibility: if is_pub {
                Visibility::Public
            } else {
                Visibility::Private
            },
        }
    }

    // ========== Helper Parsers 辅助解析器 ==========

    /// Parse generic parameters.
    /// 解析泛型参数。
    ///
    /// Syntax: `<T, U: Bound, V: A + B>`
    /// 语法：`<T, U: 约束, V: A + B>`
    fn parse_generics(&mut self) -> Vec<GenericParam> {
        if !self.eat(TokenKind::Lt) {
            return Vec::new();
        }

        let mut params = Vec::new();
        loop {
            let start = self.current_span();
            let name = self.parse_ident();
            // Parse optional bounds: `T: Bound1 + Bound2`
            // 解析可选的约束：`T: Bound1 + Bound2`
            let bounds = if self.eat(TokenKind::Colon) {
                let mut bounds = vec![self.parse_type()];
                while self.eat(TokenKind::Plus) {
                    bounds.push(self.parse_type());
                }
                bounds
            } else {
                Vec::new()
            };
            let end = self.previous_span();

            params.push(GenericParam {
                name,
                bounds,
                span: start.merge(end),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        self.expect(TokenKind::Gt);
        params
    }

    /// Parse function parameters.
    /// 解析函数参数。
    ///
    /// Syntax: `pattern [: type], ...`
    /// 语法：`模式 [: 类型], ...`
    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return params;
        }

        loop {
            // Check for trailing comma before RParen
            // 检查右括号前的尾随逗号
            if self.check(TokenKind::RParen) {
                break;
            }

            let start = self.current_span();
            let is_lazy = self.eat(TokenKind::Lazy);
            let pattern = self.parse_pattern();
            // Type annotation is optional - inferred if not provided
            // 类型注解是可选的 - 如果未提供则推断
            let ty = if self.eat(TokenKind::Colon) {
                self.parse_type()
            } else {
                Type {
                    kind: TypeKind::Infer,
                    span: self.current_span(),
                }
            };
            let end = self.previous_span();

            params.push(Param {
                pattern,
                ty,
                is_lazy,
                span: start.merge(end),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        params
    }

    /// Parse struct field definitions.
    /// 解析结构体字段定义。
    ///
    /// Syntax: `name: type [= default], ...`
    /// 语法：`名称: 类型 [= 默认值], ...`
    fn parse_field_defs(&mut self) -> Vec<FieldDef> {
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let start = self.current_span();
            let name = self.parse_ident();
            self.expect(TokenKind::Colon);
            let ty = self.parse_type();
            // Parse optional default value
            // 解析可选的默认值
            let default = if self.eat(TokenKind::Eq) {
                Some(self.parse_expr())
            } else {
                None
            };
            let end = self.previous_span();

            fields.push(FieldDef {
                name,
                ty,
                default,
                span: start.merge(end),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        fields
    }

    /// Parse enum variants.
    /// 解析枚举变体。
    ///
    /// Variants can be: unit, tuple, or record.
    /// 变体可以是：单元、元组或记录。
    fn parse_variants(&mut self) -> Vec<Variant> {
        let mut variants = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let start = self.current_span();
            let name = self.parse_ident();

            // Parse variant kind: unit, tuple, or record
            // 解析变体类型：单元、元组或记录
            let kind = if self.eat(TokenKind::LParen) {
                // Tuple variant: Variant(Type1, Type2)
                // 元组变体：Variant(Type1, Type2)
                let mut types = Vec::new();
                if !self.check(TokenKind::RParen) {
                    loop {
                        types.push(self.parse_type());
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen);
                VariantKind::Tuple(types)
            } else if self.eat(TokenKind::HashLBrace) {
                // Record variant: Variant #{ field: Type }
                // 记录变体：Variant #{ 字段: 类型 }
                let fields = self.parse_field_defs();
                self.expect(TokenKind::RBrace);
                VariantKind::Record(fields)
            } else {
                // Unit variant: Variant
                // 单元变体：Variant
                VariantKind::Unit
            };

            let end = self.previous_span();
            variants.push(Variant {
                name,
                kind,
                span: start.merge(end),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        variants
    }

    /// Parse a trait item (method signature).
    /// 解析特征项（方法签名）。
    fn parse_trait_item(&mut self) -> TraitItem {
        let start = self.current_span();
        self.expect(TokenKind::Fn);
        let name = self.parse_ident();
        let generics = self.parse_generics();
        self.expect(TokenKind::LParen);
        let params = self.parse_params();
        self.expect(TokenKind::RParen);

        let return_type = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };

        // Parse optional default implementation
        // 解析可选的默认实现
        let default = if self.eat(TokenKind::Eq) {
            Some(self.parse_expr())
        } else {
            None
        };

        self.expect(TokenKind::Semicolon);
        let end = self.previous_span();

        TraitItem {
            name,
            generics,
            params,
            return_type,
            default,
            span: start.merge(end),
        }
    }

    /// Parse an impl item (method implementation).
    /// 解析 impl 项（方法实现）。
    fn parse_impl_item(&mut self) -> ImplItem {
        let start = self.current_span();
        self.expect(TokenKind::Fn);
        let name = self.parse_ident();
        let generics = self.parse_generics();
        self.expect(TokenKind::LParen);
        let params = self.parse_params();
        self.expect(TokenKind::RParen);

        let return_type = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };

        self.expect(TokenKind::Eq);
        let body = self.parse_expr();
        self.expect(TokenKind::Semicolon);
        let end = self.previous_span();

        ImplItem {
            name,
            generics,
            params,
            return_type,
            body,
            span: start.merge(end),
        }
    }

    /// Parse an associated type definition in a trait.
    /// 解析特征中的关联类型定义。
    ///
    /// Syntax: `type Name [: Bounds] [= Default];`
    /// 语法：`type 名称 [: 约束] [= 默认值];`
    fn parse_assoc_type_def(&mut self) -> AssocTypeDef {
        let start = self.current_span();
        self.expect(TokenKind::Type);
        let name = self.parse_ident();

        // Parse bounds: `type Item: Eq + Show`
        // 解析约束：`type Item: Eq + Show`
        let bounds = if self.eat(TokenKind::Colon) {
            let mut bounds = vec![self.parse_type()];
            while self.eat(TokenKind::Plus) {
                bounds.push(self.parse_type());
            }
            bounds
        } else {
            Vec::new()
        };

        // Parse default: `type Item = Int`
        // 解析默认值：`type Item = Int`
        let default = if self.eat(TokenKind::Eq) {
            Some(self.parse_type())
        } else {
            None
        };

        self.expect(TokenKind::Semicolon);
        let end = self.previous_span();

        AssocTypeDef {
            name,
            bounds,
            default,
            span: start.merge(end),
        }
    }

    /// Parse an associated type implementation in an impl block.
    /// 解析 impl 块中的关联类型实现。
    ///
    /// Syntax: `type Name = Type;`
    /// 语法：`type 名称 = 类型;`
    fn parse_assoc_type_impl(&mut self) -> AssocTypeImpl {
        let start = self.current_span();
        self.expect(TokenKind::Type);
        let name = self.parse_ident();
        self.expect(TokenKind::Eq);
        let ty = self.parse_type();
        self.expect(TokenKind::Semicolon);
        let end = self.previous_span();

        AssocTypeImpl {
            name,
            ty,
            span: start.merge(end),
        }
    }

    /// Parse an identifier.
    /// 解析标识符。
    fn parse_ident(&mut self) -> Ident {
        let span = self.current_span();
        match self.current_kind() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ident::new(name, span)
            }
            _ => {
                self.error("expected identifier");
                Ident::new("", span)
            }
        }
    }

    // ========== Expression Parsing 表达式解析 ==========
    //
    // Expressions are parsed using operator precedence climbing.
    // 表达式使用运算符优先级爬升法解析。
    //
    // Precedence (lowest to highest) 优先级（从低到高）:
    // 1. Pipe: |>              管道
    // 2. Merge: //             合并
    // 3. Coalesce: ??          空值合并
    // 4. Or: ||                逻辑或
    // 5. And: &&               逻辑与
    // 6. Comparison: == != < <= > >=  比较
    // 7. Concat: ++            连接
    // 8. Additive: + -         加减
    // 9. Multiplicative: * / % 乘除取模
    // 10. Power: ^             幂运算
    // 11. Unary: ! -           一元运算
    // 12. Postfix: . [] ()     后缀运算

    /// Parse an expression.
    /// 解析表达式。
    fn parse_expr(&mut self) -> Expr {
        self.parse_pipe_expr()
    }

    /// Parse pipe expression: expr |> expr
    /// 解析管道表达式：expr |> expr
    fn parse_pipe_expr(&mut self) -> Expr {
        let mut left = self.parse_merge_expr();

        while self.eat(TokenKind::PipeGt) {
            let right = self.parse_merge_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinOp::Pipe,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse merge expression: expr // expr
    /// 解析合并表达式：expr // expr
    fn parse_merge_expr(&mut self) -> Expr {
        let mut left = self.parse_coalesce_expr();

        while self.eat(TokenKind::SlashSlash) {
            let right = self.parse_coalesce_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinOp::Merge,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse coalesce expression: expr ?? expr
    /// 解析空值合并表达式：expr ?? expr
    fn parse_coalesce_expr(&mut self) -> Expr {
        let mut left = self.parse_or_expr();

        while self.eat(TokenKind::QuestionQuestion) {
            let right = self.parse_or_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Coalesce {
                    value: Box::new(left),
                    default: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse logical or expression: expr || expr
    /// 解析逻辑或表达式：expr || expr
    fn parse_or_expr(&mut self) -> Expr {
        let mut left = self.parse_and_expr();

        while self.eat(TokenKind::OrOr) {
            let right = self.parse_and_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse logical and expression: expr && expr
    /// 解析逻辑与表达式：expr && expr
    fn parse_and_expr(&mut self) -> Expr {
        let mut left = self.parse_comparison_expr();

        while self.eat(TokenKind::AndAnd) {
            let right = self.parse_comparison_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse comparison expression: expr (== | != | < | <= | > | >=) expr
    /// 解析比较表达式：expr (== | != | < | <= | > | >=) expr
    fn parse_comparison_expr(&mut self) -> Expr {
        let mut left = self.parse_concat_expr();

        loop {
            let op = match self.current_kind() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Ne,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::LtEq => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::GtEq => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_concat_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse concatenation expression: expr ++ expr
    /// 解析连接表达式：expr ++ expr
    fn parse_concat_expr(&mut self) -> Expr {
        let mut left = self.parse_additive_expr();

        while self.eat(TokenKind::PlusPlus) {
            let right = self.parse_additive_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinOp::Concat,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse additive expression: expr (+ | -) expr
    /// 解析加法表达式：expr (+ | -) expr
    fn parse_additive_expr(&mut self) -> Expr {
        let mut left = self.parse_multiplicative_expr();

        loop {
            let op = match self.current_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse multiplicative expression: expr (* | / | %) expr
    /// 解析乘法表达式：expr (* | / | %) expr
    fn parse_multiplicative_expr(&mut self) -> Expr {
        let mut left = self.parse_power_expr();

        loop {
            let op = match self.current_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power_expr();
            let span = left.span.merge(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }

        left
    }

    /// Parse power expression: expr ^ expr (right associative)
    /// 解析幂表达式：expr ^ expr（右结合）
    fn parse_power_expr(&mut self) -> Expr {
        let left = self.parse_unary_expr();

        if self.eat(TokenKind::Caret) {
            // Right associative: 2^3^4 = 2^(3^4)
            // 右结合：2^3^4 = 2^(3^4)
            let right = self.parse_power_expr();
            let span = left.span.merge(right.span);
            Expr::new(
                ExprKind::Binary {
                    op: BinOp::Pow,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )
        } else {
            left
        }
    }

    /// Parse unary expression: (! | -) expr
    /// 解析一元表达式：(! | -) expr
    fn parse_unary_expr(&mut self) -> Expr {
        let start = self.current_span();

        // Logical not: !expr
        // 逻辑非：!expr
        if self.eat(TokenKind::Bang) {
            let operand = self.parse_unary_expr();
            let span = start.merge(operand.span);
            return Expr::new(
                ExprKind::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                },
                span,
            );
        }

        // Negation: -expr
        // 取负：-expr
        if self.eat(TokenKind::Minus) {
            let operand = self.parse_unary_expr();
            let span = start.merge(operand.span);
            return Expr::new(
                ExprKind::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                },
                span,
            );
        }

        self.parse_postfix_expr()
    }

    /// Parse postfix expression: expr (. | ?. | [] | () | ?)
    /// 解析后缀表达式：expr (. | ?. | [] | () | ?)
    fn parse_postfix_expr(&mut self) -> Expr {
        let mut expr = self.parse_primary_expr();

        loop {
            if self.eat(TokenKind::Question) {
                // Try operator: expr?
                // 尝试运算符：expr?
                let span = expr.span.merge(self.previous_span());
                expr = Expr::new(ExprKind::Try(Box::new(expr)), span);
            } else if self.eat(TokenKind::Dot) {
                // Field access or method call: expr.field or expr.method()
                // 字段访问或方法调用：expr.field 或 expr.method()
                if let TokenKind::Int(n) = self.current_kind() {
                    // Tuple index: expr.0
                    // 元组索引：expr.0
                    let n = *n as u32;
                    self.advance();
                    let span = expr.span.merge(self.previous_span());
                    expr = Expr::new(
                        ExprKind::TupleIndex {
                            base: Box::new(expr),
                            index: n,
                        },
                        span,
                    );
                } else {
                    let field = self.parse_ident();
                    if self.check(TokenKind::LParen) {
                        // Method call: expr.method(args)
                        // 方法调用：expr.method(args)
                        self.advance();
                        let args = self.parse_args();
                        self.expect(TokenKind::RParen);
                        let span = expr.span.merge(self.previous_span());
                        expr = Expr::new(
                            ExprKind::MethodCall {
                                receiver: Box::new(expr),
                                method: field,
                                args,
                            },
                            span,
                        );
                    } else {
                        // Field access: expr.field
                        // 字段访问：expr.field
                        let span = expr.span.merge(field.span);
                        expr = Expr::new(
                            ExprKind::Field {
                                base: Box::new(expr),
                                field,
                            },
                            span,
                        );
                    }
                }
            } else if self.eat(TokenKind::QuestionDot) {
                // Safe field access: expr?.field
                // 安全字段访问：expr?.field
                let field = self.parse_ident();
                let span = expr.span.merge(field.span);
                expr = Expr::new(
                    ExprKind::SafeField {
                        base: Box::new(expr),
                        field,
                    },
                    span,
                );
            } else if self.eat(TokenKind::LBracket) {
                // Index access: expr[index]
                // 索引访问：expr[index]
                let index = self.parse_expr();
                self.expect(TokenKind::RBracket);
                let span = expr.span.merge(self.previous_span());
                expr = Expr::new(
                    ExprKind::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                    },
                    span,
                );
            } else if self.check(TokenKind::LParen) {
                // Function call: expr(args)
                // 函数调用：expr(args)
                self.advance();
                let args = self.parse_args();
                self.expect(TokenKind::RParen);
                let span = expr.span.merge(self.previous_span());
                expr = Expr::new(
                    ExprKind::Call {
                        func: Box::new(expr),
                        args,
                    },
                    span,
                );
            } else if self.check(TokenKind::HashLBrace) {
                // Function call with record argument: func #{ ... }
                // 带记录参数的函数调用：func #{ ... }
                let record = self.parse_record();
                let span = expr.span.merge(record.span);
                expr = Expr::new(
                    ExprKind::Call {
                        func: Box::new(expr),
                        args: vec![record],
                    },
                    span,
                );
            } else {
                break;
            }
        }

        expr
    }

    /// Parse a primary expression.
    /// 解析基本表达式。
    ///
    /// Primary expressions are the "atoms" of the expression grammar.
    /// 基本表达式是表达式语法的"原子"。
    fn parse_primary_expr(&mut self) -> Expr {
        let start = self.current_span();

        match self.current_kind().clone() {
            // Literals 字面量
            TokenKind::Int(n) => {
                self.advance();
                Expr::new(ExprKind::Int(n), start)
            }
            TokenKind::Float(f) => {
                self.advance();
                Expr::new(ExprKind::Float(f), start)
            }
            TokenKind::String(s) => {
                self.advance();
                Expr::new(ExprKind::String(s), start)
            }
            TokenKind::Char(c) => {
                self.advance();
                Expr::new(ExprKind::Char(c), start)
            }
            TokenKind::True => {
                self.advance();
                Expr::new(ExprKind::Bool(true), start)
            }
            TokenKind::False => {
                self.advance();
                Expr::new(ExprKind::Bool(false), start)
            }
            // Identifier or path
            // 标识符或路径
            TokenKind::Ident(_) => self.parse_ident_expr(),
            // Handle 'self' as a variable expression in method bodies
            // 在方法体中将 'self' 作为变量表达式处理
            TokenKind::SelfLower => {
                self.advance();
                let ident = Ident {
                    name: "self".to_string(),
                    span: start,
                };
                Expr::new(ExprKind::Var(ident), start)
            }
            // Parenthesized expression or tuple
            // 括号表达式或元组
            TokenKind::LParen => self.parse_paren_or_tuple(),
            // List literal or comprehension
            // 列表字面量或列表推导
            TokenKind::LBracket => self.parse_list(),
            // Record literal
            // 记录字面量
            TokenKind::HashLBrace => self.parse_record(),
            // Block expression
            // 块表达式
            TokenKind::LBrace => self.parse_block(),
            // If expression
            // if 表达式
            TokenKind::If => self.parse_if(),
            // Match expression
            // match 表达式
            TokenKind::Match => self.parse_match(),
            // Lambda expression
            // Lambda 表达式
            TokenKind::Fn => self.parse_lambda(),
            // Lazy expression
            // 惰性表达式
            TokenKind::Lazy => {
                self.advance();
                let expr = self.parse_expr();
                let span = start.merge(expr.span);
                Expr::new(ExprKind::Lazy(Box::new(expr)), span)
            }
            // Interpolated string
            // 插值字符串
            TokenKind::InterpolatedStart => self.parse_interpolated_string(),
            // Path literal
            // 路径字面量
            TokenKind::PathLit(p) => {
                self.advance();
                Expr::new(ExprKind::PathLit(p), start)
            }
            _ => {
                self.error("expected expression");
                self.recover_expr()
            }
        }
    }

    /// Parse an identifier expression (variable or path).
    /// 解析标识符表达式（变量或路径）。
    fn parse_ident_expr(&mut self) -> Expr {
        let start = self.current_span();
        let first = self.parse_ident();

        // Check for path: a.b.c
        // 检查是否为路径：a.b.c
        if self.check(TokenKind::Dot) {
            let mut path = vec![first];
            while self.eat(TokenKind::Dot) {
                if let TokenKind::Ident(_) = self.current_kind() {
                    path.push(self.parse_ident());
                } else {
                    break;
                }
            }
            let end = self.previous_span();
            Expr::new(ExprKind::Path(path), start.merge(end))
        } else {
            Expr::new(ExprKind::Var(first.clone()), first.span)
        }
    }

    /// Parse a parenthesized expression or tuple.
    /// 解析括号表达式或元组。
    ///
    /// Syntax: `()` (unit), `(expr)` (grouping), `(a, b, ...)` (tuple)
    /// 语法：`()` (单元), `(expr)` (分组), `(a, b, ...)` (元组)
    fn parse_paren_or_tuple(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // (

        // Empty parentheses: unit type
        // 空括号：单元类型
        if self.eat(TokenKind::RParen) {
            return Expr::new(ExprKind::Unit, start.merge(self.previous_span()));
        }

        let first = self.parse_expr();

        // Check for tuple (comma after first element)
        // 检查是否为元组（第一个元素后有逗号）
        if self.eat(TokenKind::Comma) {
            let mut elements = vec![first];
            if !self.check(TokenKind::RParen) {
                loop {
                    elements.push(self.parse_expr());
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen);
            let span = start.merge(self.previous_span());
            Expr::new(ExprKind::Tuple(elements), span)
        } else {
            // Just a parenthesized expression
            // 只是一个括号表达式
            self.expect(TokenKind::RParen);
            first
        }
    }

    /// Parse a list literal or list comprehension.
    /// 解析列表字面量或列表推导。
    ///
    /// Syntax: `[a, b, ...]` (list) or `[expr | generators]` (comprehension)
    /// 语法：`[a, b, ...]` (列表) 或 `[expr | 生成器]` (推导)
    fn parse_list(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // [

        // Empty list
        // 空列表
        if self.eat(TokenKind::RBracket) {
            return Expr::new(
                ExprKind::List(Vec::new()),
                start.merge(self.previous_span()),
            );
        }

        let first = self.parse_expr();

        // Check for list comprehension
        // 检查是否为列表推导
        if self.eat(TokenKind::Pipe) {
            let generators = self.parse_generators();
            self.expect(TokenKind::RBracket);
            let span = start.merge(self.previous_span());
            return Expr::new(
                ExprKind::ListComp {
                    body: Box::new(first),
                    generators,
                },
                span,
            );
        }

        // Regular list
        // 普通列表
        let mut elements = vec![first];
        while self.eat(TokenKind::Comma) {
            if self.check(TokenKind::RBracket) {
                break;
            }
            elements.push(self.parse_expr());
        }

        self.expect(TokenKind::RBracket);
        let span = start.merge(self.previous_span());
        Expr::new(ExprKind::List(elements), span)
    }

    /// Parse list comprehension generators.
    /// 解析列表推导的生成器。
    ///
    /// Syntax: `pattern <- iter [, condition]`
    /// 语法：`模式 <- 迭代器 [, 条件]`
    fn parse_generators(&mut self) -> Vec<Generator> {
        let mut generators = Vec::new();

        loop {
            let start = self.current_span();
            let pattern = self.parse_pattern();

            // Check for `<-`
            // 检查 `<-`
            self.expect(TokenKind::Lt);
            self.expect(TokenKind::Minus);

            let iter = self.parse_expr();

            // Optional condition
            // 可选的条件
            let condition = if self.eat(TokenKind::Comma) {
                if matches!(self.current_kind(), TokenKind::Ident(_)) {
                    Some(self.parse_expr())
                } else {
                    None
                }
            } else {
                None
            };

            let end = self.previous_span();
            generators.push(Generator {
                pattern,
                iter,
                condition,
                span: start.merge(end),
            });

            if !self.check(TokenKind::Comma) {
                break;
            }
        }

        generators
    }

    /// Parse a record literal or record update.
    /// 解析记录字面量或记录更新。
    ///
    /// Syntax: `#{ field = value, ... }` or `#{ base | field = value }`
    /// 语法：`#{ 字段 = 值, ... }` 或 `#{ 基础 | 字段 = 值 }`
    fn parse_record(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // #{

        // Empty record
        // 空记录
        if self.eat(TokenKind::RBrace) {
            return Expr::new(
                ExprKind::Record(Vec::new()),
                start.merge(self.previous_span()),
            );
        }

        // Check for record update: #{ base | field = value }
        // 检查记录更新：#{ base | field = value }
        let first_ident = self.parse_ident();

        if self.eat(TokenKind::Pipe) {
            // Record update syntax
            // 记录更新语法
            let base = Expr::new(ExprKind::Var(first_ident.clone()), first_ident.span);
            let fields = self.parse_record_fields();
            self.expect(TokenKind::RBrace);
            let span = start.merge(self.previous_span());
            return Expr::new(
                ExprKind::RecordUpdate {
                    base: Box::new(base),
                    fields,
                },
                span,
            );
        }

        // Regular record
        // 普通记录
        let first_value = if self.eat(TokenKind::Eq) {
            Some(self.parse_expr())
        } else {
            None
        };

        let first_field = RecordField {
            name: first_ident.clone(),
            value: first_value,
            span: first_ident.span,
        };

        let mut fields = vec![first_field];
        while self.eat(TokenKind::Comma) {
            if self.check(TokenKind::RBrace) {
                break;
            }
            let name = self.parse_ident();
            let value = if self.eat(TokenKind::Eq) {
                Some(self.parse_expr())
            } else {
                // Shorthand: `#{ x }` is equivalent to `#{ x = x }`
                // 简写：`#{ x }` 等价于 `#{ x = x }`
                None
            };
            fields.push(RecordField {
                span: name.span,
                name,
                value,
            });
        }

        self.expect(TokenKind::RBrace);
        let span = start.merge(self.previous_span());
        Expr::new(ExprKind::Record(fields), span)
    }

    /// Parse record fields for record update.
    /// 解析记录更新的字段。
    fn parse_record_fields(&mut self) -> Vec<RecordField> {
        let mut fields = Vec::new();
        loop {
            let name = self.parse_ident();
            let value = if self.eat(TokenKind::Eq) {
                Some(self.parse_expr())
            } else {
                None
            };
            fields.push(RecordField {
                span: name.span,
                name,
                value,
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        fields
    }

    /// Parse a block expression.
    /// 解析块表达式。
    ///
    /// Syntax: `{ stmt; ... expr }`
    /// 语法：`{ 语句; ... 表达式 }`
    fn parse_block(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // {

        // Save previous recovery mode and set block-appropriate mode
        // 保存之前的恢复模式并设置适合块的模式
        let prev_recovery = self.recovery_mode;
        self.recovery_mode = RecoveryMode::Delimiter(DelimiterKind::Brace);

        let mut stmts = Vec::new();
        let mut final_expr = None;

        while !self.check(TokenKind::RBrace) && !self.at_end() {
            if self.check(TokenKind::Let) {
                // Let statement
                // let 语句
                let stmt_start = self.current_span();
                self.advance();
                let pattern = self.parse_pattern();
                let ty = if self.eat(TokenKind::Colon) {
                    Some(self.parse_type())
                } else {
                    None
                };
                if !self.expect_recover(TokenKind::Eq, RecoveryMode::Statement) {
                    let _ = self.recover_stmt();
                    continue;
                }
                let value = self.parse_expr();
                self.expect_recover(TokenKind::Semicolon, RecoveryMode::Statement);
                let stmt_end = self.previous_span();
                stmts.push(Stmt {
                    kind: StmtKind::Let { pattern, ty, value },
                    span: stmt_start.merge(stmt_end),
                });
            } else {
                // Expression (statement or final expression)
                // 表达式（语句或最终表达式）
                let expr = self.parse_expr();
                if self.eat(TokenKind::Semicolon) {
                    // Expression statement
                    // 表达式语句
                    let stmt_span = expr.span;
                    stmts.push(Stmt {
                        kind: StmtKind::Expr(expr),
                        span: stmt_span,
                    });
                } else {
                    // Final expression (no semicolon)
                    // 最终表达式（无分号）
                    final_expr = Some(Box::new(expr));
                    break;
                }
            }
        }

        self.expect(TokenKind::RBrace);
        self.recovery_mode = prev_recovery;
        let span = start.merge(self.previous_span());
        Expr::new(
            ExprKind::Block {
                stmts,
                expr: final_expr,
            },
            span,
        )
    }

    /// Parse an if expression.
    /// 解析 if 表达式。
    ///
    /// Syntax: `if condition then then_branch else else_branch`
    /// 语法：`if 条件 then 真分支 else 假分支`
    fn parse_if(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // if

        let condition = self.parse_expr();
        self.expect(TokenKind::Then);
        let then_branch = self.parse_expr();
        self.expect(TokenKind::Else);
        let else_branch = self.parse_expr();

        let span = start.merge(else_branch.span);
        Expr::new(
            ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            span,
        )
    }

    /// Parse a match expression.
    /// 解析 match 表达式。
    ///
    /// Syntax: `match scrutinee { pattern [if guard] => body, ... }`
    /// 语法：`match 被匹配值 { 模式 [if 守卫] => 分支体, ... }`
    fn parse_match(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // match

        let scrutinee = self.parse_expr();
        self.expect(TokenKind::LBrace);

        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let arm_start = self.current_span();
            let pattern = self.parse_pattern();

            // Optional guard
            // 可选的守卫条件
            let guard = if self.eat(TokenKind::If) {
                Some(self.parse_expr())
            } else {
                None
            };

            self.expect(TokenKind::Arrow);
            let body = self.parse_expr();

            let arm_end = self.previous_span();
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: arm_start.merge(arm_end),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        self.expect(TokenKind::RBrace);
        let span = start.merge(self.previous_span());
        Expr::new(
            ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span,
        )
    }

    /// Parse a lambda expression.
    /// 解析 lambda 表达式。
    ///
    /// Syntax: `fn(params) body`
    /// 语法：`fn(参数) 函数体`
    fn parse_lambda(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // fn

        self.expect(TokenKind::LParen);
        let params = self.parse_lambda_params();
        self.expect(TokenKind::RParen);

        let body = self.parse_expr();
        let span = start.merge(body.span);

        Expr::new(
            ExprKind::Lambda {
                params,
                body: Box::new(body),
            },
            span,
        )
    }

    /// Parse lambda parameters.
    /// 解析 lambda 参数。
    fn parse_lambda_params(&mut self) -> Vec<LambdaParam> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return params;
        }

        loop {
            let start = self.current_span();
            let pattern = self.parse_pattern();
            let ty = if self.eat(TokenKind::Colon) {
                Some(self.parse_type())
            } else {
                None
            };
            let end = self.previous_span();

            params.push(LambdaParam {
                pattern,
                ty,
                span: start.merge(end),
            });

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        params
    }

    /// Parse an interpolated string.
    /// 解析插值字符串。
    ///
    /// Syntax: `"text ${expr} more text"`
    /// 语法：`"文本 ${表达式} 更多文本"`
    fn parse_interpolated_string(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // InterpolatedStart

        let mut parts = Vec::new();

        loop {
            match self.current_kind().clone() {
                TokenKind::InterpolatedPart(s) => {
                    // Literal text part
                    // 字面文本部分
                    self.advance();
                    parts.push(StringPart::Literal(s));
                }
                TokenKind::InterpolationStart => {
                    // Expression interpolation: ${expr}
                    // 表达式插值：${expr}
                    self.advance();
                    let expr = self.parse_expr();
                    parts.push(StringPart::Expr(expr));
                    if !self.eat(TokenKind::InterpolationEnd) {
                        self.error("expected `}` to close interpolation");
                    }
                }
                TokenKind::InterpolatedEnd => {
                    // End of interpolated string
                    // 插值字符串结束
                    self.advance();
                    break;
                }
                TokenKind::Eof => {
                    self.error("unterminated interpolated string");
                    break;
                }
                _ => {
                    self.error("unexpected token in interpolated string");
                    self.advance();
                }
            }
        }

        let span = start.merge(self.previous_span());
        Expr::new(ExprKind::Interpolated(parts), span)
    }

    /// Parse function call arguments.
    /// 解析函数调用参数。
    fn parse_args(&mut self) -> Vec<Expr> {
        self.parse_comma_list(TokenKind::RParen, |parser| Some(parser.parse_expr()))
    }

    // ========== Pattern Parsing 模式解析 ==========

    /// Parse a pattern.
    /// 解析模式。
    fn parse_pattern(&mut self) -> Pattern {
        self.parse_or_pattern()
    }

    /// Parse an or-pattern: pattern | pattern
    /// 解析或模式：模式 | 模式
    fn parse_or_pattern(&mut self) -> Pattern {
        let mut patterns = vec![self.parse_binding_pattern()];

        while self.eat(TokenKind::Pipe) {
            patterns.push(self.parse_binding_pattern());
        }

        if patterns.len() == 1 {
            // Safe: we just checked len == 1
            // 安全：我们刚检查过 len == 1
            patterns.pop().expect("patterns has exactly one element")
        } else {
            // Safe: patterns has at least 2 elements (len != 1 and we parsed at least one)
            // 安全：patterns 至少有 2 个元素（len != 1 且我们至少解析了一个）
            let span = patterns
                .first()
                .expect("patterns is non-empty")
                .span
                .merge(patterns.last().expect("patterns is non-empty").span);
            Pattern::new(PatternKind::Or(patterns), span)
        }
    }

    /// Parse a binding pattern: name @ pattern
    /// 解析绑定模式：名称 @ 模式
    fn parse_binding_pattern(&mut self) -> Pattern {
        let start = self.current_span();

        if let TokenKind::Ident(_) = self.current_kind() {
            let ident = self.parse_ident();

            if self.eat(TokenKind::At) {
                // Binding pattern: name @ pattern
                // 绑定模式：名称 @ 模式
                let pattern = self.parse_primary_pattern();
                let span = start.merge(pattern.span);
                return Pattern::new(
                    PatternKind::Binding {
                        name: ident,
                        pattern: Box::new(pattern),
                    },
                    span,
                );
            }

            if self.check(TokenKind::LParen) {
                // Constructor pattern: Name(args)
                // 构造器模式：Name(args)
                self.advance(); // consume '('
                let mut args = Vec::new();
                if !self.check(TokenKind::RParen) {
                    loop {
                        args.push(self.parse_pattern());
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen);
                let span = start.merge(self.previous_span());
                return Pattern::new(
                    PatternKind::Constructor {
                        path: vec![ident],
                        args,
                    },
                    span,
                );
            }

            // Just a variable pattern
            // 只是一个变量模式
            return Pattern::new(PatternKind::Var(ident.clone()), ident.span);
        }

        self.parse_primary_pattern()
    }

    /// Parse a primary pattern.
    /// 解析基本模式。
    fn parse_primary_pattern(&mut self) -> Pattern {
        let start = self.current_span();

        match self.current_kind().clone() {
            // Wildcard pattern: _
            // 通配符模式：_
            TokenKind::Ident(name) if name == "_" => {
                self.advance();
                Pattern::new(PatternKind::Wildcard, start)
            }
            // Handle 'self' as a special variable pattern in method parameters
            // 在方法参数中将 'self' 作为特殊变量模式处理
            TokenKind::SelfLower => {
                self.advance();
                let ident = Ident {
                    name: "self".to_string(),
                    span: start,
                };
                Pattern::new(PatternKind::Var(ident), start)
            }
            // Variable or constructor pattern
            // 变量或构造器模式
            TokenKind::Ident(_) => {
                let ident = self.parse_ident();
                if self.check(TokenKind::LParen) {
                    // Constructor pattern
                    // 构造器模式
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(TokenKind::RParen) {
                        loop {
                            args.push(self.parse_pattern());
                            if !self.eat(TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RParen);
                    let span = start.merge(self.previous_span());
                    Pattern::new(
                        PatternKind::Constructor {
                            path: vec![ident],
                            args,
                        },
                        span,
                    )
                } else {
                    Pattern::new(PatternKind::Var(ident.clone()), ident.span)
                }
            }
            // Literal patterns
            // 字面量模式
            TokenKind::Int(n) => {
                self.advance();
                Pattern::new(PatternKind::Literal(LiteralPattern::Int(n)), start)
            }
            TokenKind::Float(f) => {
                self.advance();
                Pattern::new(PatternKind::Literal(LiteralPattern::Float(f)), start)
            }
            TokenKind::String(s) => {
                self.advance();
                Pattern::new(PatternKind::Literal(LiteralPattern::String(s)), start)
            }
            TokenKind::Char(c) => {
                self.advance();
                Pattern::new(PatternKind::Literal(LiteralPattern::Char(c)), start)
            }
            TokenKind::True => {
                self.advance();
                Pattern::new(PatternKind::Literal(LiteralPattern::Bool(true)), start)
            }
            TokenKind::False => {
                self.advance();
                Pattern::new(PatternKind::Literal(LiteralPattern::Bool(false)), start)
            }
            // Tuple pattern: (a, b, ...)
            // 元组模式：(a, b, ...)
            TokenKind::LParen => {
                self.advance();
                if self.eat(TokenKind::RParen) {
                    return Pattern::new(
                        PatternKind::Tuple(Vec::new()),
                        start.merge(self.previous_span()),
                    );
                }
                let first = self.parse_pattern();
                if self.eat(TokenKind::Comma) {
                    let mut elements = vec![first];
                    if !self.check(TokenKind::RParen) {
                        loop {
                            elements.push(self.parse_pattern());
                            if !self.eat(TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RParen);
                    let span = start.merge(self.previous_span());
                    Pattern::new(PatternKind::Tuple(elements), span)
                } else {
                    self.expect(TokenKind::RParen);
                    first
                }
            }
            // List pattern: [a, b, ..rest, c]
            // 列表模式：[a, b, ..rest, c]
            TokenKind::LBracket => {
                self.advance();
                if self.eat(TokenKind::RBracket) {
                    return Pattern::new(
                        PatternKind::List(Vec::new()),
                        start.merge(self.previous_span()),
                    );
                }

                let mut init = Vec::new();
                let mut rest = None;
                let mut tail = Vec::new();
                let mut seen_rest = false;

                loop {
                    if self.eat(TokenKind::DotDot) {
                        // Rest pattern: ..name or just ..
                        // 剩余模式：..名称 或仅 ..
                        if !seen_rest {
                            if let TokenKind::Ident(_) = self.current_kind() {
                                rest = Some(Box::new(self.parse_pattern()));
                            }
                            seen_rest = true;
                        }
                    } else {
                        let pattern = self.parse_pattern();
                        if seen_rest {
                            tail.push(pattern);
                        } else {
                            init.push(pattern);
                        }
                    }

                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }

                self.expect(TokenKind::RBracket);
                let span = start.merge(self.previous_span());

                if seen_rest {
                    Pattern::new(PatternKind::ListRest { init, rest, tail }, span)
                } else {
                    Pattern::new(PatternKind::List(init), span)
                }
            }
            // Record pattern: #{ field, field = pattern, .. }
            // 记录模式：#{ 字段, 字段 = 模式, .. }
            TokenKind::HashLBrace => {
                self.advance();
                let mut fields = Vec::new();
                let mut rest = false;

                if !self.check(TokenKind::RBrace) {
                    loop {
                        if self.eat(TokenKind::DotDot) {
                            // Rest pattern: ignore remaining fields
                            // 剩余模式：忽略剩余字段
                            rest = true;
                            break;
                        }

                        let name = self.parse_ident();
                        let pattern = if self.eat(TokenKind::Eq) {
                            Some(self.parse_pattern())
                        } else {
                            None
                        };
                        fields.push(RecordPatternField {
                            span: name.span,
                            name,
                            pattern,
                        });

                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(TokenKind::RBrace);
                let span = start.merge(self.previous_span());
                Pattern::new(PatternKind::Record { fields, rest }, span)
            }
            _ => {
                self.error("expected pattern");
                // Advance to prevent infinite loop on unexpected tokens
                // 前进以防止在意外 token 上无限循环
                self.advance();
                Pattern::new(PatternKind::Wildcard, start)
            }
        }
    }

    // ========== Type Parsing 类型解析 ==========

    /// Parse a type.
    /// 解析类型。
    fn parse_type(&mut self) -> Type {
        self.parse_function_type()
    }

    /// Parse a function type: Type -> Type
    /// 解析函数类型：类型 -> 类型
    fn parse_function_type(&mut self) -> Type {
        let first = self.parse_primary_type();

        if self.eat(TokenKind::Arrow) {
            let result = self.parse_function_type();
            let span = first.span.merge(result.span);
            Type::new(
                TypeKind::Function {
                    params: vec![first],
                    result: Box::new(result),
                },
                span,
            )
        } else {
            first
        }
    }

    /// Parse a primary type.
    /// 解析基本类型。
    fn parse_primary_type(&mut self) -> Type {
        let start = self.current_span();

        match self.current_kind().clone() {
            // Named type with optional path and generics: a.b.Type<T, U>
            // 带可选路径和泛型的命名类型：a.b.Type<T, U>
            TokenKind::Ident(_) => {
                let mut path = vec![self.parse_ident()];
                while self.eat(TokenKind::Dot) {
                    path.push(self.parse_ident());
                }

                // Parse optional generic arguments
                // 解析可选的泛型参数
                let args = if self.eat(TokenKind::Lt) {
                    let mut args = vec![self.parse_type()];
                    while self.eat(TokenKind::Comma) {
                        args.push(self.parse_type());
                    }
                    self.expect(TokenKind::Gt);
                    args
                } else {
                    Vec::new()
                };

                let span = start.merge(self.previous_span());
                Type::new(TypeKind::Named { path, args }, span)
            }
            // Unit type or tuple type: () or (A, B, ...)
            // 单元类型或元组类型：() 或 (A, B, ...)
            TokenKind::LParen => {
                self.advance();
                if self.eat(TokenKind::RParen) {
                    return Type::new(TypeKind::Unit, start.merge(self.previous_span()));
                }

                let first = self.parse_type();
                if self.eat(TokenKind::Comma) {
                    let mut elements = vec![first];
                    loop {
                        elements.push(self.parse_type());
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen);
                    let span = start.merge(self.previous_span());
                    Type::new(TypeKind::Tuple(elements), span)
                } else {
                    self.expect(TokenKind::RParen);
                    first
                }
            }
            // Record type: #{ field: Type, ... }
            // 记录类型：#{ 字段: 类型, ... }
            TokenKind::HashLBrace => {
                self.advance();
                let mut fields = Vec::new();

                if !self.check(TokenKind::RBrace) {
                    loop {
                        let name = self.parse_ident();
                        self.expect(TokenKind::Colon);
                        let ty = self.parse_type();
                        fields.push(RecordTypeField {
                            span: name.span.merge(ty.span),
                            name,
                            ty,
                        });

                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(TokenKind::RBrace);
                let span = start.merge(self.previous_span());
                Type::new(TypeKind::Record(fields), span)
            }
            _ => {
                self.error("expected type");
                Type::new(TypeKind::Infer, start)
            }
        }
    }

    // ========== Token Helpers Token 辅助方法 ==========

    /// Get the current token.
    /// 获取当前 token。
    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    /// Get the kind of the current token.
    /// 获取当前 token 的类型。
    fn current_kind(&self) -> &TokenKind {
        &self.current().kind
    }

    /// Get the span of the current token.
    /// 获取当前 token 的位置信息。
    fn current_span(&self) -> Span {
        self.current().span
    }

    /// Get the span of the previous token.
    /// 获取前一个 token 的位置信息。
    fn previous_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::DUMMY
        }
    }

    /// Check if we're at the end of the token stream.
    /// 检查是否已到达 token 流末尾。
    fn at_end(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    /// Check if the current token matches the given kind.
    /// 检查当前 token 是否匹配给定类型。
    fn check(&self, kind: TokenKind) -> bool {
        std::mem::discriminant(self.current_kind()) == std::mem::discriminant(&kind)
    }

    /// Advance to the next token.
    /// 前进到下一个 token。
    fn advance(&mut self) {
        if !self.at_end() {
            self.pos += 1;
        }
    }

    /// Consume the current token if it matches the given kind.
    /// 如果当前 token 匹配给定类型则消耗它。
    ///
    /// Returns true if the token was consumed.
    /// 如果 token 被消耗则返回 true。
    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Expect the current token to match the given kind.
    /// 期望当前 token 匹配给定类型。
    ///
    /// Reports an error if the token doesn't match.
    /// 如果 token 不匹配则报告错误。
    fn expect(&mut self, kind: TokenKind) {
        if !self.eat(kind.clone()) {
            self.error(&format!("expected {:?}", kind));
        }
    }

    /// Report a parse error at the current position.
    /// 在当前位置报告解析错误。
    fn error(&mut self, message: &str) {
        let span = self.current_span();
        self.diagnostics.push(
            Diagnostic::error(DiagnosticKind::Parser, span, message)
                .with_code(ErrorCode::UnexpectedToken)
                .with_label(Label::new(span, "here")),
        );
    }

    // ========== Error Recovery 错误恢复 ==========

    /// Synchronize to the next statement boundary.
    /// 同步到下一个语句边界。
    ///
    /// This skips tokens until we find a synchronization point.
    /// 跳过 token 直到找到同步点。
    fn synchronize(&mut self) {
        // Must advance at least once to avoid infinite loop when we can't parse current token
        // 必须至少前进一次以避免在无法解析当前 token 时无限循环
        let mut advanced = false;

        while !self.at_end() {
            // If we just passed a statement-ending token, we're at a statement boundary
            // 如果我们刚经过一个语句结束 token，我们就在语句边界
            if self.pos > 0 {
                let prev = &self.tokens[self.pos - 1].kind;
                if is_stmt_end(prev) {
                    // Only return if we've advanced, otherwise we'd loop forever
                    // 只有在已前进的情况下才返回，否则会永远循环
                    if advanced {
                        return;
                    }
                }
            }

            // If current token starts a new statement, stop here
            // 如果当前 token 开始一个新语句，在此停止
            if is_stmt_start(self.current_kind()) {
                return;
            }

            // Update delimiter tracking and advance
            // 更新定界符跟踪并前进
            let kind = self.current_kind().clone();
            self.delimiter_stack.update(&kind);
            self.advance();
            advanced = true;
        }
    }

    /// Check if we're at the end of a statement.
    /// 检查是否在语句末尾。
    fn at_stmt_end(&self) -> bool {
        STMT_ENDS
            .iter()
            .any(|k| std::mem::discriminant(self.current_kind()) == std::mem::discriminant(k))
    }

    /// Synchronize to a specific token or statement boundary.
    /// 同步到特定 token 或语句边界。
    fn synchronize_to(&mut self, target: TokenKind) {
        while !self.at_end() {
            if self.check(target.clone()) {
                return;
            }

            // Stop at statement-ending tokens
            // 在语句结束 token 处停止
            if self.at_stmt_end() {
                return;
            }

            // Also stop at statement boundaries (sync tokens like keywords)
            // 也在语句边界停止（如关键字等同步 token）
            if is_sync_token(self.current_kind()) {
                return;
            }

            let kind = self.current_kind().clone();
            self.delimiter_stack.update(&kind);
            self.advance();
        }
    }

    /// Skip until we find a closing delimiter, respecting nesting.
    /// 跳过直到找到闭合定界符，同时尊重嵌套。
    fn skip_to_closing_delimiter(&mut self, kind: DelimiterKind) {
        let target = kind.closing_token();
        let mut depth = 1;

        while !self.at_end() && depth > 0 {
            let current = self.current_kind().clone();

            if current == kind.opening_token() {
                depth += 1;
            } else if current == target {
                depth -= 1;
                if depth == 0 {
                    return; // Don't consume the closing delimiter / 不消耗闭合定界符
                }
            }

            self.advance();
        }
    }

    /// Try to recover from an error in an expression context.
    /// 尝试从表达式上下文中的错误恢复。
    fn recover_expr(&mut self) -> Expr {
        let span = self.current_span();
        self.synchronize();
        // Return a placeholder error expression (unit)
        // 返回一个占位符错误表达式（单元）
        Expr::new(ExprKind::Unit, span)
    }

    /// Try to recover from an error in a statement context.
    /// 尝试从语句上下文中的错误恢复。
    fn recover_stmt(&mut self) -> Option<Stmt> {
        self.synchronize();
        None
    }

    /// Expect a token, with recovery on failure.
    /// 期望一个 token，失败时进行恢复。
    fn expect_recover(&mut self, kind: TokenKind, recovery: RecoveryMode) -> bool {
        if self.eat(kind.clone()) {
            true
        } else {
            self.error(&format!("expected {:?}", kind));
            match recovery {
                RecoveryMode::Statement => self.synchronize(),
                RecoveryMode::Expression => {
                    // Skip to common expression terminators
                    // 跳过到常见的表达式终结符
                    self.synchronize_to(TokenKind::Semicolon);
                }
                RecoveryMode::Delimiter(delim) => {
                    self.skip_to_closing_delimiter(delim);
                }
                RecoveryMode::None => {}
            }
            false
        }
    }

    /// Parse a comma-separated list with error recovery.
    /// 解析带有错误恢复的逗号分隔列表。
    fn parse_comma_list<T, F>(&mut self, closing: TokenKind, mut parse_item: F) -> Vec<T>
    where
        F: FnMut(&mut Self) -> Option<T>,
    {
        let mut items = Vec::new();

        while !self.check(closing.clone()) && !self.at_end() {
            if let Some(item) = parse_item(self) {
                items.push(item);
            } else {
                // Recovery: skip to comma or closing delimiter
                // 恢复：跳过到逗号或闭合定界符
                while !self.check(TokenKind::Comma)
                    && !self.check(closing.clone())
                    && !self.at_end()
                {
                    self.advance();
                }
            }

            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_associated_types() {
        let source = r#"
trait Iterator {
    type Item;
    fn next(self) -> Option<Self.Item>;
};

trait Container {
    type Item: Show;
    type Error = String;
};

impl<T> Iterator for List<T> {
    type Item = T;
    fn next(self) -> Option<T> = None;
};
"#;

        let lexer = neve_lexer::Lexer::new(source);
        let (tokens, _diags) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let source_file = parser.parse_file();

        // Check for no errors
        // 检查是否无错误
        let diagnostics = parser.diagnostics();
        if !diagnostics.is_empty() {
            for diag in &diagnostics {
                eprintln!("Error: {:?}", diag);
            }
            panic!("Parser should not produce errors");
        }

        // Verify we parsed 3 items
        // 验证我们解析了 3 个项
        assert_eq!(source_file.items.len(), 3);

        // Check first trait (Iterator)
        // 检查第一个特征（Iterator）
        if let ItemKind::Trait(trait_def) = &source_file.items[0].kind {
            assert_eq!(trait_def.name.name, "Iterator");
            assert_eq!(trait_def.assoc_types.len(), 1);
            assert_eq!(trait_def.assoc_types[0].name.name, "Item");
            assert_eq!(trait_def.assoc_types[0].bounds.len(), 0);
            assert!(trait_def.assoc_types[0].default.is_none());
            assert_eq!(trait_def.items.len(), 1);
        } else {
            panic!("First item should be a trait");
        }

        // Check second trait (Container)
        // 检查第二个特征（Container）
        if let ItemKind::Trait(trait_def) = &source_file.items[1].kind {
            assert_eq!(trait_def.name.name, "Container");
            assert_eq!(trait_def.assoc_types.len(), 2);

            // First assoc type with bound
            // 第一个带约束的关联类型
            assert_eq!(trait_def.assoc_types[0].name.name, "Item");
            assert_eq!(trait_def.assoc_types[0].bounds.len(), 1);
            assert!(trait_def.assoc_types[0].default.is_none());

            // Second assoc type with default
            // 第二个带默认值的关联类型
            assert_eq!(trait_def.assoc_types[1].name.name, "Error");
            assert_eq!(trait_def.assoc_types[1].bounds.len(), 0);
            assert!(trait_def.assoc_types[1].default.is_some());
        } else {
            panic!("Second item should be a trait");
        }

        // Check impl block
        // 检查 impl 块
        if let ItemKind::Impl(impl_def) = &source_file.items[2].kind {
            assert_eq!(impl_def.assoc_type_impls.len(), 1);
            assert_eq!(impl_def.assoc_type_impls[0].name.name, "Item");
            assert_eq!(impl_def.items.len(), 1);
        } else {
            panic!("Third item should be an impl");
        }
    }
}
