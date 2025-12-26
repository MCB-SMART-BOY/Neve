//! Document management for the LSP server.

use neve_common::Span;
use neve_parser::parse;
use neve_syntax::SourceFile;
use neve_hir::{Module, lower};
use neve_typeck::check;

use crate::symbol_index::SymbolIndex;

/// A document being edited.
#[derive(Debug)]
pub struct Document {
    /// The document URI.
    pub uri: String,
    /// The document content.
    pub content: String,
    /// The parsed AST (if available).
    pub ast: Option<SourceFile>,
    /// The lowered HIR (if available).
    pub hir: Option<Module>,
    /// Symbol index for navigation features.
    pub symbol_index: Option<SymbolIndex>,
    /// Diagnostics for this document.
    pub diagnostics: Vec<Diagnostic>,
}

/// A diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    /// Create a severity from a numeric level (for external use).
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
    pub fn update(&mut self, content: String) {
        self.content = content;
        self.diagnostics.clear();
        self.analyze();
    }
    
    /// Analyze the document.
    fn analyze(&mut self) {
        // Parse
        let (ast, parse_diagnostics) = parse(&self.content);
        
        for diag in parse_diagnostics {
            self.diagnostics.push(Diagnostic {
                span: diag.span,
                message: diag.message.clone(),
                severity: DiagnosticSeverity::Error,
            });
        }
        
        // Build symbol index for navigation
        self.symbol_index = Some(SymbolIndex::from_ast(&ast));
        
        self.ast = Some(ast.clone());
        
        // HIR lowering
        let hir = lower(&ast);
        self.hir = Some(hir.clone());
        
        // Type checking
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
    pub fn offset_at(&self, line: u32, column: u32) -> usize {
        let mut offset = 0;
        for (i, line_content) in self.content.lines().enumerate() {
            if i == line as usize {
                return offset + column as usize;
            }
            offset += line_content.len() + 1; // +1 for newline
        }
        offset
    }
    
    /// Get the line and column for an offset.
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_new() {
        let doc = Document::new(
            "file:///test.neve".to_string(),
            "let x = 1;".to_string(),
        );
        assert!(doc.ast.is_some());
    }
    
    #[test]
    fn test_position_at() {
        let doc = Document::new(
            "file:///test.neve".to_string(),
            "let x = 1;\nlet y = 2;".to_string(),
        );
        assert_eq!(doc.position_at(0), (0, 0));
        assert_eq!(doc.position_at(11), (1, 0));
    }
}
