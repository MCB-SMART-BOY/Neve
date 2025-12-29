//! Diagnostic types and builders.
//! 诊断类型和构建器。

use crate::ErrorCode;
use neve_common::Span;

/// Severity level of a diagnostic.
/// 诊断的严重程度级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Error - compilation cannot continue. / 错误 - 编译无法继续。
    Error,
    /// Warning - potential issue. / 警告 - 潜在问题。
    Warning,
    /// Note - informational message. / 注释 - 信息性消息。
    Note,
}

/// Kind of diagnostic for categorization.
/// 诊断的分类类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// Lexer diagnostic. / 词法分析器诊断。
    Lexer,
    /// Parser diagnostic. / 语法分析器诊断。
    Parser,
    /// Type checking diagnostic. / 类型检查诊断。
    Type,
    /// Evaluation diagnostic. / 求值诊断。
    Eval,
    /// Module loading diagnostic. / 模块加载诊断。
    Module,
}

/// A labeled span within a diagnostic.
/// 诊断中带标签的源码范围。
#[derive(Debug, Clone)]
pub struct Label {
    /// The source span. / 源码范围。
    pub span: Span,
    /// The label message. / 标签信息。
    pub message: String,
}

impl Label {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// A diagnostic message with optional labels, notes, and help.
/// 诊断消息，可包含标签、注释和帮助信息。
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level. / 严重程度级别。
    pub severity: Severity,
    /// Diagnostic category. / 诊断分类。
    pub kind: DiagnosticKind,
    /// Error code for lookup. / 用于查找的错误代码。
    pub code: Option<ErrorCode>,
    /// Main message. / 主要信息。
    pub message: String,
    /// Primary span. / 主要源码范围。
    pub span: Span,
    /// Additional labeled spans. / 附加的带标签范围。
    pub labels: Vec<Label>,
    /// Additional notes. / 附加注释。
    pub notes: Vec<String>,
    /// Help suggestion. / 帮助建议。
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(kind: DiagnosticKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            kind,
            code: None,
            message: message.into(),
            span,
            labels: vec![],
            notes: vec![],
            help: None,
        }
    }

    pub fn warning(kind: DiagnosticKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            kind,
            code: None,
            message: message.into(),
            span,
            labels: vec![],
            notes: vec![],
            help: None,
        }
    }

    pub fn with_code(mut self, code: ErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
