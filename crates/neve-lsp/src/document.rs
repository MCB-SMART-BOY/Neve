//! Document management for the LSP server.
//! LSP 服务器的文档管理。
//!
//! Handles parsing, analysis, and diagnostics for open documents.
//! 处理打开文档的解析、分析和诊断。

use neve_common::Span;
use neve_hir::{Module, lower};
use neve_parser::parse;
use neve_syntax::SourceFile;
use neve_typeck::check;

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
    /// Diagnostics for this document. / 此文档的诊断信息。
    pub diagnostics: Vec<Diagnostic>,
}

/// A diagnostic message.
/// 诊断消息。
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The span of the diagnostic. / 诊断的范围。
    pub span: Span,
    /// The diagnostic message. / 诊断消息。
    pub message: String,
    /// The severity of the diagnostic. / 诊断的严重程度。
    pub severity: DiagnosticSeverity,
}

/// Diagnostic severity levels.
/// 诊断严重程度级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// Error. / 错误。
    Error,
    /// Warning. / 警告。
    Warning,
    /// Information. / 信息。
    Information,
    /// Hint. / 提示。
    Hint,
}

impl DiagnosticSeverity {
    /// Create a severity from a numeric level (for external use).
    /// 从数字级别创建严重程度（用于外部使用）。
    /// 1 = Error, 2 = Warning, 3 = Information, 4 = Hint
    pub fn from_level(level: u8) -> Self {
        match level {
            1 => DiagnosticSeverity::Error,
            2 => DiagnosticSeverity::Warning,
            3 => DiagnosticSeverity::Information,
            4 => DiagnosticSeverity::Hint,
            _ => DiagnosticSeverity::Error,
        }
    }

    /// Get the numeric level for this severity.
    /// 获取此严重程度的数字级别。
    pub fn to_level(self) -> u8 {
        match self {
            DiagnosticSeverity::Error => 1,
            DiagnosticSeverity::Warning => 2,
            DiagnosticSeverity::Information => 3,
            DiagnosticSeverity::Hint => 4,
        }
    }
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
        // Parse / 解析
        let (ast, parse_diagnostics) = parse(&self.content);

        for diag in parse_diagnostics {
            self.diagnostics.push(Diagnostic {
                span: diag.span,
                message: diag.message.clone(),
                severity: DiagnosticSeverity::Error,
            });
        }

        // Build symbol index for navigation
        // 构建用于导航的符号索引
        self.symbol_index = Some(SymbolIndex::from_ast(&ast));

        self.ast = Some(ast.clone());

        // HIR lowering / HIR 降级
        let hir = lower(&ast);
        self.hir = Some(hir.clone());

        // Type checking / 类型检查
        let type_diagnostics = check(&hir);
        for diag in type_diagnostics {
            self.diagnostics.push(Diagnostic {
                span: diag.span,
                message: diag.message.clone(),
                severity: DiagnosticSeverity::Error,
            });
        }
    }

    /// Get the offset for a line and column.
    /// 获取行列对应的偏移量。
    pub fn offset_at(&self, line: u32, column: u32) -> usize {
        let mut offset = 0;
        for (i, line_content) in self.content.lines().enumerate() {
            if i == line as usize {
                return offset + column as usize;
            }
            offset += line_content.len() + 1; // +1 for newline / +1 用于换行符
        }
        offset
    }

    /// Get the line and column for an offset.
    /// 获取偏移量对应的行列。
    pub fn position_at(&self, offset: usize) -> (u32, u32) {
        let mut line = 0;
        let mut col = 0;

        for (i, c) in self.content.chars().enumerate() {
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
}
