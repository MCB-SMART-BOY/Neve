//! Document management for the LSP server.
//! LSP 服务器的文档管理。
//!
//! Handles parsing, analysis, and diagnostics for open documents.
//! 处理打开文档的解析、分析和诊断。

use std::collections::HashMap;

use neve_common::Span;
use neve_frontend::{Diagnostic, Module, SourceFile, analyze_source};
use neve_hir::{ItemKind as HirItemKind, Ty, TyKind};
use neve_syntax::{self as ast, PatternKind as AstPatternKind};
use neve_typeck::{TypeChecker, format_type};

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
            (ast::ItemKind::Let(def), HirItemKind::Fn(_)) => {
                if let AstPatternKind::Var(ident) = &def.pattern.kind
                    && let Some(ty) = checker.global_type(hir_item.id)
                {
                    hovers.insert(
                        ident.span,
                        format!("let {}: {}", ident.name, format_type(&ty)),
                    );
                }
            }
            (ast::ItemKind::Fn(def), HirItemKind::Fn(_)) => {
                if let Some(ty) = checker.global_type(hir_item.id) {
                    hovers.insert(
                        def.name.span,
                        format!("fn {}: {}", def.name.name, format_type(&ty)),
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
                            format!("fn {}: {}", ast_item.name.name, format_type(&ty)),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    hovers
}

fn callable_type_string(generics: &[neve_hir::GenericParam], params: &[Ty], ret: &Ty) -> String {
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

    format_type(&ty)
}
