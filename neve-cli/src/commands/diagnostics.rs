//! Shared CLI helpers for frontend diagnostic emission.
//! frontend 诊断发射的 CLI 共享辅助函数。

use neve_diagnostic::emit;
use neve_frontend::{
    Diagnostic, LoadedSnippetModule, ProgramDiagnosticModule, SessionDisplayError,
    SessionLoadedDiagnostics,
};
use std::path::Path;

trait SourceAttributedDiagnosticsEntry {
    fn file_path(&self) -> &Path;
    fn source(&self) -> &str;
    fn diagnostics(&self) -> &[Diagnostic];
}

impl SourceAttributedDiagnosticsEntry for ProgramDiagnosticModule {
    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl SourceAttributedDiagnosticsEntry for LoadedSnippetModule {
    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

impl SourceAttributedDiagnosticsEntry for SessionLoadedDiagnostics {
    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn source(&self) -> &str {
        &self.source
    }

    fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Emit diagnostics attributed to one source name and source text.
/// Returns the count of errors and warnings emitted.
/// 发射归属到单个源码名称与源码文本的诊断。
/// 返回发出的错误和警告计数。
pub(super) fn emit_source_diagnostics(source_name: &str, source: &str, diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        emit(source, source_name, diagnostic);
    }
}

/// Emit a Rust-style diagnostic summary line.
/// 发射 Rust 风格的诊断摘要行。
///
/// Prints something like:
/// `error: could not compile `file.neve` due to 3 previous errors`
/// or `warning: 2 warnings emitted`
#[allow(dead_code)]
pub(super) fn emit_diagnostic_summary(
    error_count: usize,
    warning_count: usize,
    filename: Option<&str>,
) {
    if error_count > 0 {
        if let Some(name) = filename {
            eprintln!(
                "error: could not compile `{}` due to {} previous error{}",
                name,
                error_count,
                if error_count == 1 { "" } else { "s" }
            );
        } else {
            eprintln!(
                "error: {} error{} found",
                error_count,
                if error_count == 1 { "" } else { "s" }
            );
        }
    }
    if warning_count > 0 {
        eprintln!(
            "warning: {} warning{} emitted",
            warning_count,
            if warning_count == 1 { "" } else { "s" }
        );
    }
}

/// Count errors and warnings in a diagnostic slice.
/// 统计诊断切片中的错误和警告数量。
#[allow(dead_code)]
pub(super) fn count_diagnostics(diagnostics: &[Diagnostic]) -> (usize, usize) {
    let mut errors = 0;
    let mut warnings = 0;
    for diag in diagnostics {
        match diag.severity {
            neve_diagnostic::Severity::Error => errors += 1,
            neve_diagnostic::Severity::Warning => warnings += 1,
            _ => {}
        }
    }
    (errors, warnings)
}

fn emit_source_attributed_entries<T: SourceAttributedDiagnosticsEntry>(entries: &[T]) {
    for entry in entries {
        let file_path = entry.file_path().display().to_string();
        emit_source_diagnostics(&file_path, entry.source(), entry.diagnostics());
    }
}

/// Emit one dependency-first sequence of program diagnostic entries.
/// 发射一组按依赖优先顺序排列的程序诊断条目。
pub(super) fn emit_program_diagnostic_entries(entries: &[ProgramDiagnosticModule]) {
    emit_source_attributed_entries(entries);
}

/// Emit one dependency-first sequence of loaded snippet diagnostic entries.
/// 发射一组按依赖优先顺序排列的 snippet 已加载依赖诊断条目。
/// Emit one dependency-first sequence of session loaded-module diagnostic entries.
/// 发射一组按依赖优先顺序排列的 session 已加载模块诊断条目。
pub(super) fn emit_session_loaded_diagnostic_entries(entries: &[SessionLoadedDiagnostics]) {
    emit_source_attributed_entries(entries);
}

/// Emit one frontend-owned REPL/session display error.
/// 发射一个 frontend 持有的 REPL/session 展示错误。
pub(super) fn emit_session_display_error(error: SessionDisplayError) {
    match error {
        SessionDisplayError::Diagnostics {
            source_name,
            source,
            diagnostics,
        } => emit_source_diagnostics(&source_name, &source, &diagnostics),
        SessionDisplayError::LoadedModules(entries) => {
            emit_session_loaded_diagnostic_entries(&entries)
        }
        SessionDisplayError::Message(message) => {
            eprintln!("{message}");
        }
    }
}
