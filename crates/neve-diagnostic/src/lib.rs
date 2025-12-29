//! Diagnostic and error reporting for Neve.
//! Neve 的诊断和错误报告。
//!
//! This crate provides beautiful error messages using ariadne.
//! 本 crate 使用 ariadne 库提供美观的错误信息。

mod codes;
mod diagnostic;

pub use codes::ErrorCode;
pub use diagnostic::{Diagnostic, DiagnosticKind, Label, Severity};

use ariadne::{ColorGenerator, Label as AriadneLabel, Report, ReportKind, Source};

/// Render a diagnostic to stderr.
/// 将诊断信息渲染到标准错误输出。
pub fn emit(source: &str, filename: &str, diagnostic: &Diagnostic) {
    let kind = match diagnostic.severity {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
        Severity::Note => ReportKind::Advice,
    };

    let mut colors = ColorGenerator::new();
    let mut report = Report::build(kind, filename, diagnostic.span.start.0 as usize)
        .with_message(&diagnostic.message);

    if let Some(code) = &diagnostic.code {
        report = report.with_code(code.as_str());
    }

    for label in &diagnostic.labels {
        let color = colors.next();
        let ariadne_label = AriadneLabel::new((filename, label.span.range()))
            .with_message(&label.message)
            .with_color(color);
        report = report.with_label(ariadne_label);
    }

    for note in &diagnostic.notes {
        report = report.with_note(note);
    }

    if let Some(help) = &diagnostic.help {
        report = report.with_help(help);
    }

    report
        .finish()
        .eprint((filename, Source::from(source)))
        .unwrap();
}
