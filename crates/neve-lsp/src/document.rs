//! Document management for the LSP server.
//! LSP 服务器的文档管理。
//!
//! Handles parsing, analysis, and diagnostics for open documents.
//! 处理打开文档的解析、分析和诊断。

use neve_frontend::{Diagnostic, Module, SourceFile, analyze_source};

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
        // Use the frontend pipeline for consistent diagnostics.
        // 使用前端管线以保持诊断一致。
        let analysis = analyze_source(&self.content);

        self.symbol_index = Some(SymbolIndex::from_ast(&analysis.ast));
        self.ast = Some(analysis.ast);
        self.hir = Some(analysis.hir);
        self.diagnostics = analysis.diagnostics;
    }

    /// Get the offset for a line and column.
    /// 获取行列对应的偏移量。
    pub fn offset_at(&self, line: u32, column: u32) -> usize {
        let mut offset = 0usize;
        let mut current_line = 0u32;

        for line_content in self.content.split('\n') {
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
            current_line += 1;
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
