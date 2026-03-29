//! Frontend analysis pipeline for Neve.
//! Neve 的前端分析管线。
//!
//! This crate provides a small, stable API to parse, lower, and type check
//! source text in a single pass for tools like the LSP and CLI.
//! 本 crate 提供稳定的 API，用于在一次流程中完成解析、降级与类型检查，
//! 便于 LSP 和 CLI 等工具复用。

pub use neve_diagnostic::Diagnostic;
use neve_hir::DefId;
pub use neve_hir::Module;
pub use neve_syntax::SourceFile;

use neve_hir::lower;
use neve_parser::parse;
use neve_typeck::TypeChecker;
use neve_common::Span;
use std::collections::HashMap;

/// Result of analyzing a single source string.
/// 单个源文本的分析结果。
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Parsed AST. / 解析后的 AST。
    pub ast: SourceFile,
    /// Lowered HIR. / 降级后的 HIR。
    pub hir: Module,
    /// Resolved method call targets for HIR evaluation.
    /// 用于 HIR 求值的方法调用解析结果。
    pub method_resolutions: HashMap<Span, DefId>,
    /// Diagnostics from parse + type check. / 解析与类型检查诊断。
    pub diagnostics: Vec<Diagnostic>,
}

/// Analyze a source string and return AST, HIR, and diagnostics.
/// 分析源文本并返回 AST、HIR 以及诊断。
pub fn analyze_source(source: &str) -> AnalysisResult {
    // Parse first so we can reuse the AST for symbol indexing.
    // 先解析以便复用 AST 做符号索引。
    let (ast, mut diagnostics) = parse(source);

    let hir = lower(&ast);
    let mut checker = TypeChecker::new();
    checker.check(&hir);
    let method_resolutions = checker.method_resolutions().clone();
    diagnostics.extend(checker.diagnostics());

    AnalysisResult {
        ast,
        hir,
        method_resolutions,
        diagnostics,
    }
}

/// Analyze an already-parsed AST.
/// 分析已解析的 AST。
pub fn analyze_ast(ast: &SourceFile) -> AnalysisResult {
    let hir = lower(ast);
    let mut checker = TypeChecker::new();
    checker.check(&hir);
    let method_resolutions = checker.method_resolutions().clone();
    let diagnostics = checker.diagnostics();

    AnalysisResult {
        ast: ast.clone(),
        hir,
        method_resolutions,
        diagnostics,
    }
}
