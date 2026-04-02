//! Frontend analysis pipeline for Neve.
//! Neve 的前端分析管线。
//!
//! This crate provides a small, stable API to parse, lower, and type check
//! source text in a single pass for tools like the LSP and CLI.
//! 本 crate 提供稳定的 API，用于在一次流程中完成解析、降级与类型检查，
//! 便于 LSP 和 CLI 等工具复用。

pub use neve_diagnostic::Diagnostic;
pub use neve_hir::Module;
use neve_hir::{DefId, ItemKind as HirItemKind, Ty, TyKind};
pub use neve_syntax::SourceFile;

use neve_common::Span;
use neve_hir::lower;
use neve_parser::parse;
use neve_typeck::{TypeChecker, format_builtin_named_type};
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

/// Format a type using names available in the given module.
/// 使用给定模块中可见的名称格式化类型。
pub fn format_type_in_module(ty: &Ty, module: &Module) -> String {
    let mut names = HashMap::new();
    for item in &module.items {
        match &item.kind {
            HirItemKind::Fn(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Struct(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Enum(def) => {
                names.insert(item.id, def.name.clone());
                for variant in &def.variants {
                    names.insert(variant.id, variant.name.clone());
                }
            }
            HirItemKind::TypeAlias(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Trait(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Impl(_) => {}
        }
    }

    format_type_with_names(ty, &names)
}

fn format_type_with_names(ty: &Ty, names: &HashMap<DefId, String>) -> String {
    match &ty.kind {
        TyKind::Int => "Int".to_string(),
        TyKind::Float => "Float".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::Char => "Char".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Unit => "()".to_string(),
        TyKind::Var(id) => format!("?{id}"),
        TyKind::Param(_, name) => name.clone(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::SelfAssoc(name) => format!("Self.{name}"),
        TyKind::Tuple(items) => {
            let parts: Vec<_> = items
                .iter()
                .map(|item| format_type_with_names(item, names))
                .collect();
            format!("({})", parts.join(", "))
        }
        TyKind::Record(fields) => {
            let parts: Vec<_> = fields
                .iter()
                .map(|(name, ty)| format!("{name}: {}", format_type_with_names(ty, names)))
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        TyKind::Fn(params, ret) => {
            let params: Vec<_> = params
                .iter()
                .map(|param| format_type_with_names(param, names))
                .collect();
            format!(
                "({}) -> {}",
                params.join(", "),
                format_type_with_names(ret, names)
            )
        }
        TyKind::Forall(params, inner) => {
            format!(
                "forall {}. {}",
                params.join(", "),
                format_type_with_names(inner, names)
            )
        }
        TyKind::Named(def_id, args) => {
            if let Some(formatted) =
                format_builtin_named_type(*def_id, args, &|arg| format_type_with_names(arg, names))
            {
                formatted
            } else if let Some(name) = names.get(def_id) {
                if args.is_empty() {
                    name.clone()
                } else {
                    let args: Vec<_> = args
                        .iter()
                        .map(|arg| format_type_with_names(arg, names))
                        .collect();
                    format!("{name}[{}]", args.join(", "))
                }
            } else {
                neve_typeck::format_type(ty)
            }
        }
        TyKind::Unknown => "_".to_string(),
    }
}
