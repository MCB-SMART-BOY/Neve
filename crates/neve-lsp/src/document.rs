//! Document management for the LSP server.
//! LSP 服务器的文档管理。
//!
//! Handles parsing, analysis, and diagnostics for open documents.
//! 处理打开文档的解析、分析和诊断。

use std::collections::HashMap;

use neve_common::Span;
use neve_frontend::{Diagnostic, Module, SourceFile, analyze_source, format_type_in_module};
use neve_hir::{
    Expr as HirExpr, ExprKind as HirExprKind, ItemKind as HirItemKind, LocalId,
    MatchArm as HirMatchArm, Param as HirParam, Pattern as HirPattern,
    PatternKind as HirPatternKind, Stmt as HirStmt, StmtKind as HirStmtKind, Ty, TyKind,
};
use neve_syntax::{self as ast, PatternKind as AstPatternKind};
use neve_typeck::TypeChecker;

use crate::symbol_index::SymbolIndex;

/// A document being edited.
/// 正在编辑的文档。
#[derive(Debug)]
pub struct Document {
    /// The document URI. / 文档 URI。
    pub uri: String,
    /// The document content. / 文档内容。
    pub content: String,
    /// The parsed AST (if available). / 解析的 AST（如果可用）。
    pub ast: Option<SourceFile>,
    /// The lowered HIR (if available). / 降级的 HIR（如果可用）。
    pub hir: Option<Module>,
    /// Symbol index for navigation features. / 用于导航功能的符号索引。
    pub symbol_index: Option<SymbolIndex>,
    /// Semantic hover content keyed by definition span.
    /// 按定义 span 存储的语义悬停内容。
    pub definition_hovers: HashMap<Span, String>,
    /// Diagnostics for this document. / 此文档的诊断信息。
    pub diagnostics: Vec<Diagnostic>,
}

impl Document {
    /// Create a new document.
    /// 创建新文档。
    pub fn new(uri: String, content: String) -> Self {
        let mut doc = Self {
            uri,
            content,
            ast: None,
            hir: None,
            symbol_index: None,
            definition_hovers: HashMap::new(),
            diagnostics: Vec::new(),
        };
        doc.analyze();
        doc
    }

    /// Update the document content.
    /// 更新文档内容。
    pub fn update(&mut self, content: String) {
        self.content = content;
        self.diagnostics.clear();
        self.analyze();
    }

    /// Analyze the document.
    /// 分析文档。
    fn analyze(&mut self) {
        // Use the frontend pipeline for consistent diagnostics.
        // 使用前端管线以保持诊断一致。
        let analysis = analyze_source(&self.content);

        self.symbol_index = Some(SymbolIndex::from_ast(&analysis.ast));
        self.definition_hovers = build_definition_hovers(&analysis.ast, &analysis.hir);
        self.ast = Some(analysis.ast);
        self.hir = Some(analysis.hir);
        self.diagnostics = analysis.diagnostics;
    }

    /// Get the offset for a line and column.
    /// 获取行列对应的偏移量。
    pub fn offset_at(&self, line: u32, column: u32) -> usize {
        let mut offset = 0usize;
        for (current_line, line_content) in self.content.split('\n').enumerate() {
            let current_line = current_line as u32;
            if current_line == line {
                let mut utf16_units = 0u32;
                for (byte_index, ch) in line_content.char_indices() {
                    let ch_units = ch.len_utf16() as u32;
                    if utf16_units + ch_units > column {
                        return offset + byte_index;
                    }
                    utf16_units += ch_units;
                }
                return offset + line_content.len();
            }

            offset += line_content.len() + 1;
        }

        offset
    }

    /// Get the line and column for an offset.
    /// 获取偏移量对应的行列。
    pub fn position_at(&self, offset: usize) -> (u32, u32) {
        let mut line = 0u32;
        let mut line_start = 0usize;

        for line_content in self.content.split('\n') {
            let line_end = line_start + line_content.len();
            if offset <= line_end {
                let mut utf16_col = 0u32;
                for (byte_index, ch) in line_content.char_indices() {
                    if line_start + byte_index >= offset {
                        break;
                    }
                    utf16_col += ch.len_utf16() as u32;
                }
                return (line, utf16_col);
            }

            line_start = line_end + 1;
            line += 1;
        }

        (line, 0)
    }
}

fn build_definition_hovers(ast: &SourceFile, hir: &Module) -> HashMap<Span, String> {
    let mut checker = TypeChecker::new();
    checker.check(hir);

    let mut hovers = HashMap::new();
    let mut hir_items = hir.items.iter();

    for ast_item in &ast.items {
        let hir_item = match &ast_item.kind {
            ast::ItemKind::Import(_) => continue,
            _ => match hir_items.next() {
                Some(item) => item,
                None => break,
            },
        };

        match (&ast_item.kind, &hir_item.kind) {
            (ast::ItemKind::Let(def), HirItemKind::Fn(hir_fn)) => {
                if let AstPatternKind::Var(ident) = &def.pattern.kind
                    && let Some(ty) = checker.global_type(hir_item.id)
                {
                    hovers.insert(
                        ident.span,
                        format!("let {}: {}", ident.name, format_type_in_module(&ty, hir)),
                    );
                }
                collect_expr_definition_hovers(&def.value, &hir_fn.body, &checker, hir, &mut hovers);
            }
            (ast::ItemKind::Fn(def), HirItemKind::Fn(_)) => {
                if let Some(ty) = checker.global_type(hir_item.id) {
                    hovers.insert(
                        def.name.span,
                        format!("fn {}: {}", def.name.name, format_type_in_module(&ty, hir)),
                    );
                }
                if let HirItemKind::Fn(hir_fn) = &hir_item.kind {
                    collect_param_definition_hovers(
                        &def.params,
                        &hir_fn.params,
                        &checker,
                        hir,
                        &mut hovers,
                    );
                    collect_expr_definition_hovers(
                        &def.body,
                        &hir_fn.body,
                        &checker,
                        hir,
                        &mut hovers,
                    );
                }
            }
            (ast::ItemKind::Trait(def), HirItemKind::Trait(hir_trait)) => {
                for (ast_item, hir_item) in def.items.iter().zip(&hir_trait.items) {
                    hovers.insert(
                        ast_item.name.span,
                        format!(
                            "fn {}: {}",
                            ast_item.name.name,
                            callable_type_string(
                                hir,
                                &hir_item.generics,
                                &hir_item.params,
                                &hir_item.return_ty
                            )
                        ),
                    );
                }
            }
            (ast::ItemKind::Impl(def), HirItemKind::Impl(hir_impl)) => {
                for (ast_item, hir_item) in def.items.iter().zip(&hir_impl.items) {
                    if let Some(ty) = checker.global_type(hir_item.id) {
                        hovers.insert(
                            ast_item.name.span,
                            format!(
                                "fn {}: {}",
                                ast_item.name.name,
                                format_type_in_module(&ty, hir)
                            ),
                        );
                    }
                    collect_param_definition_hovers(
                        &ast_item.params,
                        &hir_item.params,
                        &checker,
                        hir,
                        &mut hovers,
                    );
                    collect_expr_definition_hovers(
                        &ast_item.body,
                        &hir_item.body,
                        &checker,
                        hir,
                        &mut hovers,
                    );
                }
            }
            _ => {}
        }
    }

    hovers
}

fn callable_type_string(
    module: &Module,
    generics: &[neve_hir::GenericParam],
    params: &[Ty],
    ret: &Ty,
) -> String {
    let mut ty = Ty {
        kind: TyKind::Fn(params.to_vec(), Box::new(ret.clone())),
        span: Span::DUMMY,
    };

    if !generics.is_empty() {
        ty = Ty {
            kind: TyKind::Forall(
                generics.iter().map(|param| param.name.clone()).collect(),
                Box::new(ty),
            ),
            span: Span::DUMMY,
        };
    }

    format_type_in_module(&ty, module)
}

fn collect_param_definition_hovers(
    ast_params: &[ast::Param],
    hir_params: &[HirParam],
    checker: &TypeChecker,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    for (ast_param, hir_param) in ast_params.iter().zip(hir_params) {
        if let AstPatternKind::Var(ident) = &ast_param.pattern.kind
            && ident.name == hir_param.name
        {
            insert_local_hover(
                ident.span,
                &ident.name,
                hir_param.id,
                checker,
                module,
                hovers,
            );
        }
    }
}

fn collect_expr_definition_hovers(
    ast_expr: &ast::Expr,
    hir_expr: &HirExpr,
    checker: &TypeChecker,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    match (&ast_expr.kind, &hir_expr.kind) {
        (ast::ExprKind::Lambda { params, body }, HirExprKind::Lambda(hir_params, hir_body)) => {
            for (ast_param, hir_param) in params.iter().zip(hir_params) {
                if let AstPatternKind::Var(ident) = &ast_param.pattern.kind
                    && ident.name == hir_param.name
                {
                    insert_local_hover(
                        ident.span,
                        &ident.name,
                        hir_param.id,
                        checker,
                        module,
                        hovers,
                    );
                }
            }
            collect_expr_definition_hovers(body, hir_body, checker, module, hovers);
        }
        (
            ast::ExprKind::Block {
                stmts: ast_stmts,
                expr: ast_tail,
            },
            HirExprKind::Block(hir_stmts, hir_tail),
        ) => {
            for (ast_stmt, hir_stmt) in ast_stmts.iter().zip(hir_stmts) {
                collect_stmt_definition_hovers(ast_stmt, hir_stmt, checker, module, hovers);
            }
            if let (Some(ast_tail), Some(hir_tail)) = (ast_tail, hir_tail.as_deref()) {
                collect_expr_definition_hovers(ast_tail, hir_tail, checker, module, hovers);
            }
        }
        (
            ast::ExprKind::Let {
                pattern,
                value,
                body,
                ..
            },
            HirExprKind::Let {
                pattern: hir_pattern,
                value: hir_value,
                body: hir_body,
                ..
            },
        ) => {
            collect_expr_definition_hovers(value, hir_value, checker, module, hovers);
            collect_pattern_definition_hovers(pattern, hir_pattern, checker, module, hovers);
            collect_expr_definition_hovers(body, hir_body, checker, module, hovers);
        }
        (ast::ExprKind::Match { scrutinee, arms }, HirExprKind::Match(hir_scrutinee, hir_arms)) => {
            collect_expr_definition_hovers(scrutinee, hir_scrutinee, checker, module, hovers);
            for (ast_arm, hir_arm) in arms.iter().zip(hir_arms) {
                collect_match_arm_definition_hovers(ast_arm, hir_arm, checker, module, hovers);
            }
        }
        (
            ast::ExprKind::ListComp { body, generators },
            HirExprKind::ListComp {
                body: hir_body,
                generators: hir_generators,
            },
        ) => {
            for (ast_generator, hir_generator) in generators.iter().zip(hir_generators) {
                collect_expr_definition_hovers(
                    &ast_generator.iter,
                    &hir_generator.iter,
                    checker,
                    module,
                    hovers,
                );
                collect_pattern_definition_hovers(
                    &ast_generator.pattern,
                    &hir_generator.pattern,
                    checker,
                    module,
                    hovers,
                );
                if let (Some(ast_guard), Some(hir_guard)) =
                    (&ast_generator.condition, hir_generator.condition.as_ref())
                {
                    collect_expr_definition_hovers(ast_guard, hir_guard, checker, module, hovers);
                }
            }
            collect_expr_definition_hovers(body, hir_body, checker, module, hovers);
        }
        (ast::ExprKind::Call { func, args }, HirExprKind::Call(hir_func, hir_args)) => {
            collect_expr_definition_hovers(func, hir_func, checker, module, hovers);
            for (ast_arg, hir_arg) in args.iter().zip(hir_args) {
                collect_expr_definition_hovers(ast_arg, hir_arg, checker, module, hovers);
            }
        }
        (
            ast::ExprKind::MethodCall { receiver, args, .. },
            HirExprKind::MethodCall {
                receiver: hir_receiver,
                args: hir_args,
                ..
            },
        ) => {
            collect_expr_definition_hovers(receiver, hir_receiver, checker, module, hovers);
            for (ast_arg, hir_arg) in args.iter().zip(hir_args) {
                collect_expr_definition_hovers(ast_arg, hir_arg, checker, module, hovers);
            }
        }
        (ast::ExprKind::Field { base, .. }, HirExprKind::Field(hir_base, _))
        | (ast::ExprKind::SafeField { base, .. }, HirExprKind::SafeField { base: hir_base, .. })
        | (ast::ExprKind::TupleIndex { base, .. }, HirExprKind::TupleIndex(hir_base, _))
        | (ast::ExprKind::Try(base), HirExprKind::Try(hir_base))
        | (ast::ExprKind::Lazy(base), HirExprKind::Lazy(hir_base)) => {
            collect_expr_definition_hovers(base, hir_base, checker, module, hovers);
        }
        (ast::ExprKind::Index { base, index }, HirExprKind::Call(_, hir_args))
            if hir_args.len() == 2 =>
        {
            collect_expr_definition_hovers(base, &hir_args[0], checker, module, hovers);
            collect_expr_definition_hovers(index, &hir_args[1], checker, module, hovers);
        }
        (
            ast::ExprKind::Binary { left, right, .. },
            HirExprKind::Binary(_, hir_left, hir_right),
        ) => {
            collect_expr_definition_hovers(left, hir_left, checker, module, hovers);
            collect_expr_definition_hovers(right, hir_right, checker, module, hovers);
        }
        (ast::ExprKind::Unary { operand, .. }, HirExprKind::Unary(_, hir_operand)) => {
            collect_expr_definition_hovers(operand, hir_operand, checker, module, hovers);
        }
        (
            ast::ExprKind::If {
                condition,
                then_branch,
                else_branch,
            },
            HirExprKind::If(hir_condition, hir_then, hir_else),
        ) => {
            collect_expr_definition_hovers(condition, hir_condition, checker, module, hovers);
            collect_expr_definition_hovers(then_branch, hir_then, checker, module, hovers);
            collect_expr_definition_hovers(else_branch, hir_else, checker, module, hovers);
        }
        (
            ast::ExprKind::Coalesce { value, default },
            HirExprKind::Coalesce {
                value: hir_value,
                default: hir_default,
            },
        ) => {
            collect_expr_definition_hovers(value, hir_value, checker, module, hovers);
            collect_expr_definition_hovers(default, hir_default, checker, module, hovers);
        }
        (ast::ExprKind::Record(fields), HirExprKind::Record(hir_fields)) => {
            for (ast_field, (_, hir_value)) in fields.iter().zip(hir_fields) {
                if let Some(ast_value) = &ast_field.value {
                    collect_expr_definition_hovers(ast_value, hir_value, checker, module, hovers);
                }
            }
        }
        (
            ast::ExprKind::RecordUpdate { base, fields },
            HirExprKind::Binary(_, hir_base, hir_update),
        ) => {
            collect_expr_definition_hovers(base, hir_base, checker, module, hovers);
            if let HirExprKind::Record(hir_fields) = &hir_update.kind {
                for (ast_field, (_, hir_value)) in fields.iter().zip(hir_fields) {
                    if let Some(ast_value) = &ast_field.value {
                        collect_expr_definition_hovers(
                            ast_value, hir_value, checker, module, hovers,
                        );
                    }
                }
            }
        }
        (ast::ExprKind::List(items), HirExprKind::List(hir_items))
        | (ast::ExprKind::Tuple(items), HirExprKind::Tuple(hir_items)) => {
            for (ast_item, hir_item) in items.iter().zip(hir_items) {
                collect_expr_definition_hovers(ast_item, hir_item, checker, module, hovers);
            }
        }
        (ast::ExprKind::Interpolated(parts), HirExprKind::Interpolated(hir_parts)) => {
            for (ast_part, hir_part) in parts.iter().zip(hir_parts) {
                if let (ast::StringPart::Expr(ast_expr), neve_hir::StringPart::Expr(hir_expr)) =
                    (ast_part, hir_part)
                {
                    collect_expr_definition_hovers(ast_expr, hir_expr, checker, module, hovers);
                }
            }
        }
        _ => {}
    }
}

fn collect_stmt_definition_hovers(
    ast_stmt: &ast::Stmt,
    hir_stmt: &HirStmt,
    checker: &TypeChecker,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    match (&ast_stmt.kind, &hir_stmt.kind) {
        (
            ast::StmtKind::Let { pattern, value, .. },
            HirStmtKind::Let(local_id, name, _, hir_value),
        ) => {
            collect_expr_definition_hovers(value, hir_value, checker, module, hovers);
            match &pattern.kind {
                AstPatternKind::Var(ident) if ident.name == *name => {
                    insert_local_hover(ident.span, &ident.name, *local_id, checker, module, hovers);
                }
                AstPatternKind::Binding { name: ident, .. } if ident.name == *name => {
                    insert_local_hover(ident.span, &ident.name, *local_id, checker, module, hovers);
                }
                _ => {}
            }
        }
        (ast::StmtKind::Expr(ast_expr), HirStmtKind::Expr(hir_expr)) => {
            collect_expr_definition_hovers(ast_expr, hir_expr, checker, module, hovers);
        }
        _ => {}
    }
}

fn collect_match_arm_definition_hovers(
    ast_arm: &ast::MatchArm,
    hir_arm: &HirMatchArm,
    checker: &TypeChecker,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    collect_pattern_definition_hovers(&ast_arm.pattern, &hir_arm.pattern, checker, module, hovers);
    if let (Some(ast_guard), Some(hir_guard)) = (&ast_arm.guard, hir_arm.guard.as_ref()) {
        collect_expr_definition_hovers(ast_guard, hir_guard, checker, module, hovers);
    }
    collect_expr_definition_hovers(&ast_arm.body, &hir_arm.body, checker, module, hovers);
}

fn collect_pattern_definition_hovers(
    ast_pattern: &ast::Pattern,
    hir_pattern: &HirPattern,
    checker: &TypeChecker,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    match (&ast_pattern.kind, &hir_pattern.kind) {
        (AstPatternKind::Var(ident), HirPatternKind::Var(local_id, name))
            if ident.name == *name =>
        {
            insert_local_hover(ident.span, &ident.name, *local_id, checker, module, hovers);
        }
        (
            AstPatternKind::Binding {
                name: ident,
                pattern,
            },
            HirPatternKind::Binding(local_id, name, inner),
        ) if ident.name == *name => {
            insert_local_hover(ident.span, &ident.name, *local_id, checker, module, hovers);
            collect_pattern_definition_hovers(pattern, inner, checker, module, hovers);
        }
        (AstPatternKind::Tuple(ast_patterns), HirPatternKind::Tuple(hir_patterns))
        | (AstPatternKind::List(ast_patterns), HirPatternKind::List(hir_patterns))
        | (
            AstPatternKind::Constructor {
                args: ast_patterns, ..
            },
            HirPatternKind::Constructor(_, hir_patterns),
        )
        | (AstPatternKind::Or(ast_patterns), HirPatternKind::Or(hir_patterns)) => {
            for (ast_pattern, hir_pattern) in ast_patterns.iter().zip(hir_patterns) {
                collect_pattern_definition_hovers(
                    ast_pattern,
                    hir_pattern,
                    checker,
                    module,
                    hovers,
                );
            }
        }
        (
            AstPatternKind::ListRest { init, rest, tail },
            HirPatternKind::ListRest {
                init: hir_init,
                rest: hir_rest,
                tail: hir_tail,
            },
        ) => {
            for (ast_pattern, hir_pattern) in init.iter().zip(hir_init) {
                collect_pattern_definition_hovers(
                    ast_pattern,
                    hir_pattern,
                    checker,
                    module,
                    hovers,
                );
            }
            if let (Some(ast_rest), Some(hir_rest)) = (rest.as_deref(), hir_rest.as_deref()) {
                collect_pattern_definition_hovers(ast_rest, hir_rest, checker, module, hovers);
            }
            for (ast_pattern, hir_pattern) in tail.iter().zip(hir_tail) {
                collect_pattern_definition_hovers(
                    ast_pattern,
                    hir_pattern,
                    checker,
                    module,
                    hovers,
                );
            }
        }
        (ast::PatternKind::Record { fields, .. }, HirPatternKind::Record(hir_fields)) => {
            for (ast_field, (hir_name, hir_pattern)) in fields.iter().zip(hir_fields) {
                if let Some(ast_pattern) = &ast_field.pattern {
                    collect_pattern_definition_hovers(
                        ast_pattern,
                        hir_pattern,
                        checker,
                        module,
                        hovers,
                    );
                } else if ast_field.name.name == *hir_name {
                    collect_pattern_definition_hovers(
                        &ast::Pattern {
                            kind: AstPatternKind::Var(ast_field.name.clone()),
                            span: ast_field.name.span,
                        },
                        hir_pattern,
                        checker,
                        module,
                        hovers,
                    );
                }
            }
        }
        _ => {}
    }
}

fn insert_local_hover(
    span: Span,
    name: &str,
    local_id: LocalId,
    checker: &TypeChecker,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    if let Some(ty) = checker.local_type(local_id) {
        hovers.insert(
            span,
            format!("{name}: {}", format_type_in_module(&ty, module)),
        );
    }
}
