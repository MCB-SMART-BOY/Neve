//! Document management for the LSP server.
//! LSP 服务器的文档管理。
//!
//! Handles parsing, analysis, and diagnostics for open documents.
//! 处理打开文档的解析、分析和诊断。

use std::collections::HashMap;

use neve_common::Span;
use neve_frontend::{
    Diagnostic, Module, ModuleSemantics, SourceFile, analyze_source, format_type_in_module,
    format_type_use_in_module,
};
use neve_hir::{
    Expr as HirExpr, ExprKind as HirExprKind, ItemKind as HirItemKind, LocalId,
    MatchArm as HirMatchArm, Param as HirParam, Pattern as HirPattern,
    PatternKind as HirPatternKind, Stmt as HirStmt, StmtKind as HirStmtKind, Ty, TyKind,
};
use neve_syntax::{self as ast, PatternKind as AstPatternKind};

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
    /// Canonical semantic side tables for this document.
    /// 当前文档的规范语义 side tables。
    pub semantics: Option<ModuleSemantics>,
    /// Symbol index for navigation features. / 用于导航功能的符号索引。
    pub symbol_index: Option<SymbolIndex>,
    /// Semantic hover content keyed by definition span.
    /// 按定义 span 存储的语义悬停内容。
    pub definition_hovers: HashMap<Span, String>,
    /// Semantic hover content keyed by reference/expression span.
    /// 按引用/表达式 span 存储的语义悬停内容。
    pub semantic_hovers: HashMap<Span, String>,
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
            semantics: None,
            symbol_index: None,
            definition_hovers: HashMap::new(),
            semantic_hovers: HashMap::new(),
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
        let (definition_hovers, semantic_hovers) =
            build_hover_maps(&analysis.ast, &analysis.hir, &analysis.semantics);
        self.definition_hovers = definition_hovers;
        self.semantic_hovers = semantic_hovers;
        self.ast = Some(analysis.ast);
        self.hir = Some(analysis.hir);
        self.semantics = Some(analysis.semantics);
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

    /// Find the most specific semantic hover covering the offset.
    /// 查找覆盖该偏移量的最具体语义悬停信息。
    pub fn semantic_hover_at(&self, offset: usize) -> Option<(Span, &str)> {
        self.semantic_hovers
            .iter()
            .filter(|(span, _)| {
                let start: usize = span.start.into();
                let end: usize = span.end.into();
                start <= offset && offset < end
            })
            .min_by_key(|(span, _)| span.len())
            .map(|(span, text)| (*span, text.as_str()))
    }
}

fn build_hover_maps(
    ast: &SourceFile,
    hir: &Module,
    semantics: &ModuleSemantics,
) -> (HashMap<Span, String>, HashMap<Span, String>) {
    let mut definition_hovers = HashMap::new();
    let mut semantic_hovers = HashMap::new();
    let mut hir_items = hir.items.iter();

    for ast_item in &ast.items {
        let hir_item = match &ast_item.kind {
            ast::ItemKind::Import(_) => continue,
            _ => match hir_items.next() {
                Some(item) => item,
                None => break,
            },
        };

        // Skip expression statements in hover maps
        if matches!(&ast_item.kind, ast::ItemKind::ExprStmt(_)) {
            continue;
        }

        match (&ast_item.kind, &hir_item.kind) {
            (ast::ItemKind::Let(def), HirItemKind::Fn(hir_fn)) => {
                if let AstPatternKind::Var(ident) = &def.pattern.kind
                    && let Some(ty) = semantics.global_type(hir_item.id)
                {
                    definition_hovers.insert(
                        ident.span,
                        format!("let {}: {}", ident.name, format_type_in_module(ty, hir)),
                    );
                }
                collect_expr_hovers(
                    &def.value,
                    &hir_fn.body,
                    semantics,
                    hir,
                    &mut definition_hovers,
                    &mut semantic_hovers,
                );
            }
            (ast::ItemKind::Fn(def), HirItemKind::Fn(_)) => {
                if let Some(ty) = semantics.global_type(hir_item.id) {
                    definition_hovers.insert(
                        def.name.span,
                        format!("fn {}: {}", def.name.name, format_type_in_module(ty, hir)),
                    );
                }
                if let HirItemKind::Fn(hir_fn) = &hir_item.kind {
                    collect_param_hovers(
                        &def.params,
                        &hir_fn.params,
                        semantics,
                        hir,
                        &mut definition_hovers,
                    );
                    for (ast_param, hir_param) in def.params.iter().zip(&hir_fn.params) {
                        collect_type_hovers(
                            &ast_param.ty,
                            &hir_param.ty,
                            semantics,
                            hir,
                            &mut semantic_hovers,
                        );
                    }
                    if let Some(ast_return) = &def.return_type {
                        collect_type_hovers(
                            ast_return,
                            &hir_fn.return_ty,
                            semantics,
                            hir,
                            &mut semantic_hovers,
                        );
                    }
                    collect_expr_hovers(
                        &def.body,
                        &hir_fn.body,
                        semantics,
                        hir,
                        &mut definition_hovers,
                        &mut semantic_hovers,
                    );
                }
            }
            (ast::ItemKind::Trait(def), HirItemKind::Trait(hir_trait)) => {
                for (ast_item, hir_item) in def.items.iter().zip(&hir_trait.items) {
                    definition_hovers.insert(
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
                    for (ast_param, hir_param) in ast_item.params.iter().zip(&hir_item.params) {
                        collect_type_hovers(
                            &ast_param.ty,
                            hir_param,
                            semantics,
                            hir,
                            &mut semantic_hovers,
                        );
                    }
                    if let Some(ast_return) = &ast_item.return_type {
                        collect_type_hovers(
                            ast_return,
                            &hir_item.return_ty,
                            semantics,
                            hir,
                            &mut semantic_hovers,
                        );
                    }
                }
            }
            (ast::ItemKind::Impl(def), HirItemKind::Impl(hir_impl)) => {
                for (ast_item, hir_item) in def.items.iter().zip(&hir_impl.items) {
                    if let Some(ty) = semantics.global_type(hir_item.id) {
                        definition_hovers.insert(
                            ast_item.name.span,
                            format!(
                                "fn {}: {}",
                                ast_item.name.name,
                                format_type_in_module(ty, hir)
                            ),
                        );
                    }
                    collect_param_hovers(
                        &ast_item.params,
                        &hir_item.params,
                        semantics,
                        hir,
                        &mut definition_hovers,
                    );
                    for (ast_param, hir_param) in ast_item.params.iter().zip(&hir_item.params) {
                        collect_type_hovers(
                            &ast_param.ty,
                            &hir_param.ty,
                            semantics,
                            hir,
                            &mut semantic_hovers,
                        );
                    }
                    if let Some(ast_return) = &ast_item.return_type {
                        collect_type_hovers(
                            ast_return,
                            &hir_item.return_ty,
                            semantics,
                            hir,
                            &mut semantic_hovers,
                        );
                    }
                    collect_expr_hovers(
                        &ast_item.body,
                        &hir_item.body,
                        semantics,
                        hir,
                        &mut definition_hovers,
                        &mut semantic_hovers,
                    );
                }
            }
            _ => {}
        }
    }

    (definition_hovers, semantic_hovers)
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

fn collect_param_hovers(
    ast_params: &[ast::Param],
    hir_params: &[HirParam],
    semantics: &ModuleSemantics,
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
                semantics,
                module,
                hovers,
            );
        }
    }
}

fn collect_type_hovers(
    ast_ty: &ast::Type,
    hir_ty: &Ty,
    semantics: &ModuleSemantics,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    hovers.insert(
        ast_ty.span,
        format_type_use_in_module(semantics, module, ast_ty.span, hir_ty),
    );

    match (&ast_ty.kind, &hir_ty.kind) {
        (ast::TypeKind::Named { args, .. }, TyKind::Named(_, hir_args)) => {
            for (ast_arg, hir_arg) in args.iter().zip(hir_args) {
                collect_type_hovers(ast_arg, hir_arg, semantics, module, hovers);
            }
        }
        (ast::TypeKind::Function { params, result }, TyKind::Fn(hir_params, hir_result)) => {
            for (ast_param, hir_param) in params.iter().zip(hir_params) {
                collect_type_hovers(ast_param, hir_param, semantics, module, hovers);
            }
            collect_type_hovers(result, hir_result, semantics, module, hovers);
        }
        (ast::TypeKind::Tuple(items), TyKind::Tuple(hir_items)) => {
            for (ast_item, hir_item) in items.iter().zip(hir_items) {
                collect_type_hovers(ast_item, hir_item, semantics, module, hovers);
            }
        }
        (ast::TypeKind::Record(fields), TyKind::Record(hir_fields)) => {
            for (ast_field, (_, hir_field_ty)) in fields.iter().zip(hir_fields) {
                collect_type_hovers(&ast_field.ty, hir_field_ty, semantics, module, hovers);
            }
        }
        _ => {}
    }
}

fn collect_expr_hovers(
    ast_expr: &ast::Expr,
    hir_expr: &HirExpr,
    semantics: &ModuleSemantics,
    module: &Module,
    definition_hovers: &mut HashMap<Span, String>,
    semantic_hovers: &mut HashMap<Span, String>,
) {
    insert_expression_hover(
        ast_expr.span,
        hir_expr.span,
        semantics,
        module,
        semantic_hovers,
    );

    match (&ast_expr.kind, &hir_expr.kind) {
        (ast::ExprKind::Var(ident), HirExprKind::Var(local_id)) => {
            insert_local_hover(
                ident.span,
                &ident.name,
                *local_id,
                semantics,
                module,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Var(ident), HirExprKind::Global(def_id)) => {
            insert_global_hover(
                ident.span,
                &ident.name,
                *def_id,
                semantics,
                module,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Var(ident), HirExprKind::Builtin(_)) => {
            insert_expression_hover(
                ident.span,
                hir_expr.span,
                semantics,
                module,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Lambda { params, body }, HirExprKind::Lambda(hir_params, hir_body)) => {
            for (ast_param, hir_param) in params.iter().zip(hir_params) {
                if let AstPatternKind::Var(ident) = &ast_param.pattern.kind
                    && ident.name == hir_param.name
                {
                    insert_local_hover(
                        ident.span,
                        &ident.name,
                        hir_param.id,
                        semantics,
                        module,
                        definition_hovers,
                    );
                }
            }
            collect_expr_hovers(
                body,
                hir_body,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (
            ast::ExprKind::Block {
                stmts: ast_stmts,
                expr: ast_tail,
            },
            HirExprKind::Block(hir_stmts, hir_tail),
        ) => {
            for (ast_stmt, hir_stmt) in ast_stmts.iter().zip(hir_stmts) {
                collect_stmt_hovers(
                    ast_stmt,
                    hir_stmt,
                    semantics,
                    module,
                    definition_hovers,
                    semantic_hovers,
                );
            }
            if let (Some(ast_tail), Some(hir_tail)) = (ast_tail, hir_tail.as_deref()) {
                collect_expr_hovers(
                    ast_tail,
                    hir_tail,
                    semantics,
                    module,
                    definition_hovers,
                    semantic_hovers,
                );
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
            collect_expr_hovers(
                value,
                hir_value,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            collect_pattern_definition_hovers(
                pattern,
                hir_pattern,
                semantics,
                module,
                definition_hovers,
            );
            collect_expr_hovers(
                body,
                hir_body,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Match { scrutinee, arms }, HirExprKind::Match(hir_scrutinee, hir_arms)) => {
            collect_expr_hovers(
                scrutinee,
                hir_scrutinee,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            for (ast_arm, hir_arm) in arms.iter().zip(hir_arms) {
                collect_match_arm_hovers(
                    ast_arm,
                    hir_arm,
                    semantics,
                    module,
                    definition_hovers,
                    semantic_hovers,
                );
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
                collect_expr_hovers(
                    &ast_generator.iter,
                    &hir_generator.iter,
                    semantics,
                    module,
                    definition_hovers,
                    semantic_hovers,
                );
                collect_pattern_definition_hovers(
                    &ast_generator.pattern,
                    &hir_generator.pattern,
                    semantics,
                    module,
                    definition_hovers,
                );
                if let (Some(ast_guard), Some(hir_guard)) =
                    (&ast_generator.condition, hir_generator.condition.as_ref())
                {
                    collect_expr_hovers(
                        ast_guard,
                        hir_guard,
                        semantics,
                        module,
                        definition_hovers,
                        semantic_hovers,
                    );
                }
            }
            collect_expr_hovers(
                body,
                hir_body,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Call { func, args }, HirExprKind::Call(hir_func, hir_args)) => {
            collect_expr_hovers(
                func,
                hir_func,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            for (ast_arg, hir_arg) in args.iter().zip(hir_args) {
                collect_expr_hovers(
                    ast_arg,
                    hir_arg,
                    semantics,
                    module,
                    definition_hovers,
                    semantic_hovers,
                );
            }
        }
        (
            ast::ExprKind::MethodCall {
                receiver,
                method,
                args,
            },
            HirExprKind::MethodCall {
                receiver: hir_receiver,
                args: hir_args,
                ..
            },
        ) => {
            insert_method_hover(
                method.span,
                &method.name,
                hir_expr.span,
                semantics,
                module,
                semantic_hovers,
            );
            collect_expr_hovers(
                receiver,
                hir_receiver,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            for (ast_arg, hir_arg) in args.iter().zip(hir_args) {
                collect_expr_hovers(
                    ast_arg,
                    hir_arg,
                    semantics,
                    module,
                    definition_hovers,
                    semantic_hovers,
                );
            }
        }
        (ast::ExprKind::Field { base, field }, HirExprKind::Field(hir_base, _)) => {
            insert_expression_hover(
                field.span,
                hir_expr.span,
                semantics,
                module,
                semantic_hovers,
            );
            collect_expr_hovers(
                base,
                hir_base,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (
            ast::ExprKind::SafeField { base, field },
            HirExprKind::SafeField { base: hir_base, .. },
        ) => {
            insert_expression_hover(
                field.span,
                hir_expr.span,
                semantics,
                module,
                semantic_hovers,
            );
            collect_expr_hovers(
                base,
                hir_base,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (ast::ExprKind::TupleIndex { base, .. }, HirExprKind::TupleIndex(hir_base, _))
        | (ast::ExprKind::Try(base), HirExprKind::Try(hir_base))
        | (ast::ExprKind::Lazy(base), HirExprKind::Lazy(hir_base)) => {
            collect_expr_hovers(
                base,
                hir_base,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Index { base, index }, HirExprKind::Call(_, hir_args))
            if hir_args.len() == 2 =>
        {
            collect_expr_hovers(
                base,
                &hir_args[0],
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            collect_expr_hovers(
                index,
                &hir_args[1],
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (
            ast::ExprKind::Binary { left, right, .. },
            HirExprKind::Binary(_, hir_left, hir_right),
        ) => {
            collect_expr_hovers(
                left,
                hir_left,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            collect_expr_hovers(
                right,
                hir_right,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Unary { operand, .. }, HirExprKind::Unary(_, hir_operand)) => {
            collect_expr_hovers(
                operand,
                hir_operand,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (
            ast::ExprKind::If {
                condition,
                then_branch,
                else_branch,
            },
            HirExprKind::If(hir_condition, hir_then, hir_else),
        ) => {
            collect_expr_hovers(
                condition,
                hir_condition,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            collect_expr_hovers(
                then_branch,
                hir_then,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            collect_expr_hovers(
                else_branch,
                hir_else,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (
            ast::ExprKind::Coalesce { value, default },
            HirExprKind::Coalesce {
                value: hir_value,
                default: hir_default,
            },
        ) => {
            collect_expr_hovers(
                value,
                hir_value,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            collect_expr_hovers(
                default,
                hir_default,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        (ast::ExprKind::Record(fields), HirExprKind::Record(hir_fields)) => {
            for (ast_field, (_, hir_value)) in fields.iter().zip(hir_fields) {
                if let Some(ast_value) = &ast_field.value {
                    collect_expr_hovers(
                        ast_value,
                        hir_value,
                        semantics,
                        module,
                        definition_hovers,
                        semantic_hovers,
                    );
                }
            }
        }
        (
            ast::ExprKind::RecordUpdate { base, fields },
            HirExprKind::Binary(_, hir_base, hir_update),
        ) => {
            collect_expr_hovers(
                base,
                hir_base,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            if let HirExprKind::Record(hir_fields) = &hir_update.kind {
                for (ast_field, (_, hir_value)) in fields.iter().zip(hir_fields) {
                    if let Some(ast_value) = &ast_field.value {
                        collect_expr_hovers(
                            ast_value,
                            hir_value,
                            semantics,
                            module,
                            definition_hovers,
                            semantic_hovers,
                        );
                    }
                }
            }
        }
        (ast::ExprKind::List(items), HirExprKind::List(hir_items))
        | (ast::ExprKind::Tuple(items), HirExprKind::Tuple(hir_items)) => {
            for (ast_item, hir_item) in items.iter().zip(hir_items) {
                collect_expr_hovers(
                    ast_item,
                    hir_item,
                    semantics,
                    module,
                    definition_hovers,
                    semantic_hovers,
                );
            }
        }
        (ast::ExprKind::Interpolated(parts), HirExprKind::Interpolated(hir_parts)) => {
            for (ast_part, hir_part) in parts.iter().zip(hir_parts) {
                if let (ast::StringPart::Expr(ast_expr), neve_hir::StringPart::Expr(hir_expr)) =
                    (ast_part, hir_part)
                {
                    collect_expr_hovers(
                        ast_expr,
                        hir_expr,
                        semantics,
                        module,
                        definition_hovers,
                        semantic_hovers,
                    );
                }
            }
        }
        _ => {}
    }
}

fn collect_stmt_hovers(
    ast_stmt: &ast::Stmt,
    hir_stmt: &HirStmt,
    semantics: &ModuleSemantics,
    module: &Module,
    definition_hovers: &mut HashMap<Span, String>,
    semantic_hovers: &mut HashMap<Span, String>,
) {
    match (&ast_stmt.kind, &hir_stmt.kind) {
        (
            ast::StmtKind::Let { pattern, value, .. },
            HirStmtKind::Let {
                pattern: hir_pattern,
                value: hir_value,
                ..
            },
        ) => {
            collect_expr_hovers(
                value,
                hir_value,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
            collect_pattern_definition_hovers(
                pattern,
                hir_pattern,
                semantics,
                module,
                definition_hovers,
            );
        }
        (ast::StmtKind::Expr(ast_expr), HirStmtKind::Expr(hir_expr)) => {
            collect_expr_hovers(
                ast_expr,
                hir_expr,
                semantics,
                module,
                definition_hovers,
                semantic_hovers,
            );
        }
        _ => {}
    }
}

fn collect_match_arm_hovers(
    ast_arm: &ast::MatchArm,
    hir_arm: &HirMatchArm,
    semantics: &ModuleSemantics,
    module: &Module,
    definition_hovers: &mut HashMap<Span, String>,
    semantic_hovers: &mut HashMap<Span, String>,
) {
    collect_pattern_definition_hovers(
        &ast_arm.pattern,
        &hir_arm.pattern,
        semantics,
        module,
        definition_hovers,
    );
    if let (Some(ast_guard), Some(hir_guard)) = (&ast_arm.guard, hir_arm.guard.as_ref()) {
        collect_expr_hovers(
            ast_guard,
            hir_guard,
            semantics,
            module,
            definition_hovers,
            semantic_hovers,
        );
    }
    collect_expr_hovers(
        &ast_arm.body,
        &hir_arm.body,
        semantics,
        module,
        definition_hovers,
        semantic_hovers,
    );
}

fn collect_pattern_definition_hovers(
    ast_pattern: &ast::Pattern,
    hir_pattern: &HirPattern,
    semantics: &ModuleSemantics,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    match (&ast_pattern.kind, &hir_pattern.kind) {
        (AstPatternKind::Var(ident), HirPatternKind::Var(local_id, name))
            if ident.name == *name =>
        {
            insert_local_hover(
                ident.span,
                &ident.name,
                *local_id,
                semantics,
                module,
                hovers,
            );
        }
        (
            AstPatternKind::Binding {
                name: ident,
                pattern,
            },
            HirPatternKind::Binding(local_id, name, inner),
        ) if ident.name == *name => {
            insert_local_hover(
                ident.span,
                &ident.name,
                *local_id,
                semantics,
                module,
                hovers,
            );
            collect_pattern_definition_hovers(pattern, inner, semantics, module, hovers);
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
                    semantics,
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
                    semantics,
                    module,
                    hovers,
                );
            }
            if let (Some(ast_rest), Some(hir_rest)) = (rest.as_deref(), hir_rest.as_deref()) {
                collect_pattern_definition_hovers(ast_rest, hir_rest, semantics, module, hovers);
            }
            for (ast_pattern, hir_pattern) in tail.iter().zip(hir_tail) {
                collect_pattern_definition_hovers(
                    ast_pattern,
                    hir_pattern,
                    semantics,
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
                        semantics,
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
                        semantics,
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
    semantics: &ModuleSemantics,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    if let Some(ty) = semantics.local_type(local_id) {
        hovers.insert(
            span,
            format!("{name}: {}", format_type_in_module(ty, module)),
        );
    }
}

fn insert_global_hover(
    span: Span,
    name: &str,
    def_id: neve_hir::DefId,
    semantics: &ModuleSemantics,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    if let Some(ty) = semantics.global_type(def_id) {
        hovers.insert(
            span,
            format!("{name}: {}", format_type_in_module(ty, module)),
        );
    }
}

fn insert_expression_hover(
    span: Span,
    expr_span: Span,
    semantics: &ModuleSemantics,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    if let Some(ty) = semantics.expr_type(expr_span) {
        hovers.insert(span, format_type_in_module(ty, module));
    }
}

fn insert_method_hover(
    span: Span,
    name: &str,
    expr_span: Span,
    semantics: &ModuleSemantics,
    module: &Module,
    hovers: &mut HashMap<Span, String>,
) {
    if let Some(def_id) = semantics.method_resolution(expr_span)
        && let Some(ty) = semantics.global_type(def_id)
    {
        hovers.insert(
            span,
            format!("fn {name}: {}", format_type_in_module(ty, module)),
        );
    }
}
