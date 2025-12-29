//! The Neve parser.

use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};
use neve_lexer::{Token, TokenKind};
use neve_syntax::*;

use crate::recovery::{
    DelimiterKind, DelimiterStack, RecoveryMode, STMT_ENDS, is_stmt_end, is_stmt_start,
    is_sync_token,
};

/// The Neve parser.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    /// Delimiter stack for tracking balanced delimiters
    delimiter_stack: DelimiterStack,
    /// Current recovery mode
    recovery_mode: RecoveryMode,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            delimiter_stack: DelimiterStack::new(),
            recovery_mode: RecoveryMode::Statement,
        }
    }

    pub fn diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Parse a complete source file.
    pub fn parse_file(&mut self) -> SourceFile {
        let start = self.current_span();
        let mut items = Vec::new();

        while !self.at_end() {
            // Track delimiter state for each token
            let kind = self.current_kind().clone();
            self.delimiter_stack.update(&kind);

            if let Some(item) = self.parse_item() {
                items.push(item);
            } else {
                // Error recovery: synchronize to next statement boundary
                self.synchronize();
            }
        }

        let end = self.current_span();
        SourceFile {
            items,
            span: start.merge(end),
        }
    }

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

    fn parse_impl_def(&mut self) -> ImplDef {
        let generics = self.parse_generics();
        let first_type = self.parse_type();

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

    fn parse_import_def(&mut self, is_pub: bool) -> ImportDef {
        // Parse optional path prefix (self, super, crate)
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
        let mut path = vec![self.parse_ident()];
        while self.eat(TokenKind::Dot) {
            // Check if next token is '(' for import items, not a path segment
            if matches!(self.current().kind, TokenKind::LParen) {
                break;
            }
            path.push(self.parse_ident());
        }

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

    // ========== Helpers ==========

    fn parse_generics(&mut self) -> Vec<GenericParam> {
        if !self.eat(TokenKind::Lt) {
            return Vec::new();
        }

        let mut params = Vec::new();
        loop {
            let start = self.current_span();
            let name = self.parse_ident();
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

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return params;
        }

        loop {
            // Check for trailing comma before RParen
            if self.check(TokenKind::RParen) {
                break;
            }

            let start = self.current_span();
            let is_lazy = self.eat(TokenKind::Lazy);
            let pattern = self.parse_pattern();
            // Type annotation is optional - inferred if not provided
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

    fn parse_field_defs(&mut self) -> Vec<FieldDef> {
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let start = self.current_span();
            let name = self.parse_ident();
            self.expect(TokenKind::Colon);
            let ty = self.parse_type();
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

    fn parse_variants(&mut self) -> Vec<Variant> {
        let mut variants = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let start = self.current_span();
            let name = self.parse_ident();

            let kind = if self.eat(TokenKind::LParen) {
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
                let fields = self.parse_field_defs();
                self.expect(TokenKind::RBrace);
                VariantKind::Record(fields)
            } else {
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

    fn parse_assoc_type_def(&mut self) -> AssocTypeDef {
        let start = self.current_span();
        self.expect(TokenKind::Type);
        let name = self.parse_ident();

        // Parse bounds: `type Item: Eq + Show`
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

    // ========== Expression Parsing ==========

    fn parse_expr(&mut self) -> Expr {
        self.parse_pipe_expr()
    }

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

    fn parse_power_expr(&mut self) -> Expr {
        let left = self.parse_unary_expr();

        if self.eat(TokenKind::Caret) {
            let right = self.parse_power_expr(); // Right associative
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

    fn parse_unary_expr(&mut self) -> Expr {
        let start = self.current_span();

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

    fn parse_postfix_expr(&mut self) -> Expr {
        let mut expr = self.parse_primary_expr();

        loop {
            if self.eat(TokenKind::Question) {
                let span = expr.span.merge(self.previous_span());
                expr = Expr::new(ExprKind::Try(Box::new(expr)), span);
            } else if self.eat(TokenKind::Dot) {
                if let TokenKind::Int(n) = self.current_kind() {
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
                // Support for `func #{ ... }` syntax (function call with record argument)
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

    fn parse_primary_expr(&mut self) -> Expr {
        let start = self.current_span();

        match self.current_kind().clone() {
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
            TokenKind::Ident(_) => self.parse_ident_expr(),
            // Handle 'self' as a variable expression in method bodies
            TokenKind::SelfLower => {
                self.advance();
                let ident = Ident {
                    name: "self".to_string(),
                    span: start,
                };
                Expr::new(ExprKind::Var(ident), start)
            }
            TokenKind::LParen => self.parse_paren_or_tuple(),
            TokenKind::LBracket => self.parse_list(),
            TokenKind::HashLBrace => self.parse_record(),
            TokenKind::LBrace => self.parse_block(),
            TokenKind::If => self.parse_if(),
            TokenKind::Match => self.parse_match(),
            TokenKind::Fn => self.parse_lambda(),
            TokenKind::Lazy => {
                self.advance();
                let expr = self.parse_expr();
                let span = start.merge(expr.span);
                Expr::new(ExprKind::Lazy(Box::new(expr)), span)
            }
            TokenKind::InterpolatedStart => self.parse_interpolated_string(),
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

    fn parse_ident_expr(&mut self) -> Expr {
        let start = self.current_span();
        let first = self.parse_ident();

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

    fn parse_paren_or_tuple(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // (

        if self.eat(TokenKind::RParen) {
            return Expr::new(ExprKind::Unit, start.merge(self.previous_span()));
        }

        let first = self.parse_expr();

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
            self.expect(TokenKind::RParen);
            first
        }
    }

    fn parse_list(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // [

        if self.eat(TokenKind::RBracket) {
            return Expr::new(
                ExprKind::List(Vec::new()),
                start.merge(self.previous_span()),
            );
        }

        let first = self.parse_expr();

        // Check for list comprehension
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

    fn parse_generators(&mut self) -> Vec<Generator> {
        let mut generators = Vec::new();

        loop {
            let start = self.current_span();
            let pattern = self.parse_pattern();

            // Check for `<-`
            self.expect(TokenKind::Lt);
            self.expect(TokenKind::Minus);

            let iter = self.parse_expr();

            let condition = if self.eat(TokenKind::Comma) {
                if matches!(self.current_kind(), TokenKind::Ident(_)) {
                    // Could be another generator or a condition
                    // For simplicity, treat as condition if not followed by <-
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

    fn parse_record(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // #{

        if self.eat(TokenKind::RBrace) {
            return Expr::new(
                ExprKind::Record(Vec::new()),
                start.merge(self.previous_span()),
            );
        }

        // Check for record update: #{ base | field = value }
        let first_ident = self.parse_ident();

        if self.eat(TokenKind::Pipe) {
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

    fn parse_block(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // {

        // Save previous recovery mode and set block-appropriate mode
        let prev_recovery = self.recovery_mode;
        self.recovery_mode = RecoveryMode::Delimiter(DelimiterKind::Brace);

        let mut stmts = Vec::new();
        let mut final_expr = None;

        while !self.check(TokenKind::RBrace) && !self.at_end() {
            if self.check(TokenKind::Let) {
                let stmt_start = self.current_span();
                self.advance();
                let pattern = self.parse_pattern();
                let ty = if self.eat(TokenKind::Colon) {
                    Some(self.parse_type())
                } else {
                    None
                };
                if !self.expect_recover(TokenKind::Eq, RecoveryMode::Statement) {
                    // Use recover_stmt to skip to next statement
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
                let expr = self.parse_expr();
                if self.eat(TokenKind::Semicolon) {
                    let stmt_span = expr.span;
                    stmts.push(Stmt {
                        kind: StmtKind::Expr(expr),
                        span: stmt_span,
                    });
                } else {
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

    fn parse_match(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // match

        let scrutinee = self.parse_expr();
        self.expect(TokenKind::LBrace);

        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.at_end() {
            let arm_start = self.current_span();
            let pattern = self.parse_pattern();

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

    fn parse_interpolated_string(&mut self) -> Expr {
        let start = self.current_span();
        self.advance(); // InterpolatedStart

        let mut parts = Vec::new();

        loop {
            match self.current_kind().clone() {
                TokenKind::InterpolatedPart(s) => {
                    self.advance();
                    parts.push(StringPart::Literal(s));
                }
                TokenKind::InterpolationStart => {
                    self.advance();
                    let expr = self.parse_expr();
                    parts.push(StringPart::Expr(expr));
                    if !self.eat(TokenKind::InterpolationEnd) {
                        self.error("expected `}` to close interpolation");
                    }
                }
                TokenKind::InterpolatedEnd => {
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

    fn parse_args(&mut self) -> Vec<Expr> {
        self.parse_comma_list(TokenKind::RParen, |parser| Some(parser.parse_expr()))
    }

    // ========== Pattern Parsing ==========

    fn parse_pattern(&mut self) -> Pattern {
        self.parse_or_pattern()
    }

    fn parse_or_pattern(&mut self) -> Pattern {
        let mut patterns = vec![self.parse_binding_pattern()];

        while self.eat(TokenKind::Pipe) {
            patterns.push(self.parse_binding_pattern());
        }

        if patterns.len() == 1 {
            patterns.pop().unwrap()
        } else {
            let span = patterns
                .first()
                .unwrap()
                .span
                .merge(patterns.last().unwrap().span);
            Pattern::new(PatternKind::Or(patterns), span)
        }
    }

    fn parse_binding_pattern(&mut self) -> Pattern {
        let start = self.current_span();

        if let TokenKind::Ident(_) = self.current_kind() {
            // Peek ahead to check if this is a binding pattern (name @ pattern)
            // or a constructor pattern (Name(args)) or just a variable
            let ident = self.parse_ident();

            if self.eat(TokenKind::At) {
                // Binding pattern: name @ pattern
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
            return Pattern::new(PatternKind::Var(ident.clone()), ident.span);
        }

        self.parse_primary_pattern()
    }

    fn parse_primary_pattern(&mut self) -> Pattern {
        let start = self.current_span();

        match self.current_kind().clone() {
            TokenKind::Ident(name) if name == "_" => {
                self.advance();
                Pattern::new(PatternKind::Wildcard, start)
            }
            // Handle 'self' as a special variable pattern in method parameters
            TokenKind::SelfLower => {
                self.advance();
                let ident = Ident {
                    name: "self".to_string(),
                    span: start,
                };
                Pattern::new(PatternKind::Var(ident), start)
            }
            TokenKind::Ident(_) => {
                let ident = self.parse_ident();
                if self.check(TokenKind::LParen) {
                    // Constructor pattern
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
            TokenKind::HashLBrace => {
                self.advance();
                let mut fields = Vec::new();
                let mut rest = false;

                if !self.check(TokenKind::RBrace) {
                    loop {
                        if self.eat(TokenKind::DotDot) {
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
                self.advance();
                Pattern::new(PatternKind::Wildcard, start)
            }
        }
    }

    // ========== Type Parsing ==========

    fn parse_type(&mut self) -> Type {
        self.parse_function_type()
    }

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

    fn parse_primary_type(&mut self) -> Type {
        let start = self.current_span();

        match self.current_kind().clone() {
            TokenKind::Ident(_) => {
                let mut path = vec![self.parse_ident()];
                while self.eat(TokenKind::Dot) {
                    path.push(self.parse_ident());
                }

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

    // ========== Token Helpers ==========

    fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    fn current_kind(&self) -> &TokenKind {
        &self.current().kind
    }

    fn current_span(&self) -> Span {
        self.current().span
    }

    fn previous_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span
        } else {
            Span::DUMMY
        }
    }

    fn at_end(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }

    fn check(&self, kind: TokenKind) -> bool {
        std::mem::discriminant(self.current_kind()) == std::mem::discriminant(&kind)
    }

    fn advance(&mut self) {
        if !self.at_end() {
            self.pos += 1;
        }
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) {
        if !self.eat(kind.clone()) {
            self.error(&format!("expected {:?}", kind));
        }
    }

    fn error(&mut self, message: &str) {
        let span = self.current_span();
        self.diagnostics.push(
            Diagnostic::error(DiagnosticKind::Parser, span, message)
                .with_code(ErrorCode::UnexpectedToken)
                .with_label(Label::new(span, "here")),
        );
    }

    // ========== Error Recovery ==========

    /// Synchronize to the next statement boundary.
    /// This skips tokens until we find a synchronization point.
    fn synchronize(&mut self) {
        // Must advance at least once to avoid infinite loop when we can't parse current token
        let mut advanced = false;

        while !self.at_end() {
            // If we just passed a statement-ending token, we're at a statement boundary
            if self.pos > 0 {
                let prev = &self.tokens[self.pos - 1].kind;
                if is_stmt_end(prev) {
                    // Only return if we've advanced, otherwise we'd loop forever
                    if advanced {
                        return;
                    }
                }
            }

            // If current token starts a new statement, stop here
            if is_stmt_start(self.current_kind()) {
                return;
            }

            // Update delimiter tracking and advance
            let kind = self.current_kind().clone();
            self.delimiter_stack.update(&kind);
            self.advance();
            advanced = true;
        }
    }

    /// Check if we're at the end of a statement.
    /// Uses STMT_ENDS constant for the token set.
    fn at_stmt_end(&self) -> bool {
        STMT_ENDS
            .iter()
            .any(|k| std::mem::discriminant(self.current_kind()) == std::mem::discriminant(k))
    }

    /// Synchronize to a specific token or statement boundary.
    fn synchronize_to(&mut self, target: TokenKind) {
        while !self.at_end() {
            if self.check(target.clone()) {
                return;
            }

            // Stop at statement-ending tokens
            if self.at_stmt_end() {
                return;
            }

            // Also stop at statement boundaries (sync tokens like keywords)
            if is_sync_token(self.current_kind()) {
                return;
            }

            let kind = self.current_kind().clone();
            self.delimiter_stack.update(&kind);
            self.advance();
        }
    }

    /// Skip until we find a closing delimiter, respecting nesting.
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
                    return; // Don't consume the closing delimiter
                }
            }

            self.advance();
        }
    }

    /// Try to recover from an error in an expression context.
    fn recover_expr(&mut self) -> Expr {
        let span = self.current_span();
        self.synchronize();
        // Return a placeholder error expression (unit)
        Expr::new(ExprKind::Unit, span)
    }

    /// Try to recover from an error in a statement context.
    fn recover_stmt(&mut self) -> Option<Stmt> {
        self.synchronize();
        None
    }

    /// Expect a token, with recovery on failure.
    fn expect_recover(&mut self, kind: TokenKind, recovery: RecoveryMode) -> bool {
        if self.eat(kind.clone()) {
            true
        } else {
            self.error(&format!("expected {:?}", kind));
            match recovery {
                RecoveryMode::Statement => self.synchronize(),
                RecoveryMode::Expression => {
                    // Skip to common expression terminators
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
        let diagnostics = parser.diagnostics();
        if !diagnostics.is_empty() {
            for diag in &diagnostics {
                eprintln!("Error: {:?}", diag);
            }
            panic!("Parser should not produce errors");
        }

        // Verify we parsed 3 items
        assert_eq!(source_file.items.len(), 3);

        // Check first trait (Iterator)
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
        if let ItemKind::Trait(trait_def) = &source_file.items[1].kind {
            assert_eq!(trait_def.name.name, "Container");
            assert_eq!(trait_def.assoc_types.len(), 2);

            // First assoc type with bound
            assert_eq!(trait_def.assoc_types[0].name.name, "Item");
            assert_eq!(trait_def.assoc_types[0].bounds.len(), 1);
            assert!(trait_def.assoc_types[0].default.is_none());

            // Second assoc type with default
            assert_eq!(trait_def.assoc_types[1].name.name, "Error");
            assert_eq!(trait_def.assoc_types[1].bounds.len(), 0);
            assert!(trait_def.assoc_types[1].default.is_some());
        } else {
            panic!("Second item should be a trait");
        }

        // Check impl block
        if let ItemKind::Impl(impl_def) = &source_file.items[2].kind {
            assert_eq!(impl_def.assoc_type_impls.len(), 1);
            assert_eq!(impl_def.assoc_type_impls[0].name.name, "Item");
            assert_eq!(impl_def.items.len(), 1);
        } else {
            panic!("Third item should be an impl");
        }
    }
}
