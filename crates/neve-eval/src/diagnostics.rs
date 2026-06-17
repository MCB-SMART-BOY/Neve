//! Conversion from EvalError to Diagnostic.
//! 将 EvalError 转换为 Diagnostic。
//!
//! This module bridges the evaluator's internal error type to
//! the structured diagnostic system for human-friendly output.
//! 本模块将求值器的内部错误类型桥接到结构化诊断系统，
//! 以提供人性化的输出。

use crate::eval::EvalError;
use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};

/// Attach a source span to an eval error and convert it to a Diagnostic.
/// 将源码 span 附加到求值错误并转换为 Diagnostic。
///
/// This is the main entry point for converting runtime errors into
/// structured, user-friendly compiler diagnostics.
/// 这是将运行时错误转换为结构化、用户友好编译器诊断的主要入口点。
pub fn eval_error_to_diagnostic(error: &EvalError, span: Span) -> Diagnostic {
    match error {
        EvalError::UnboundVariable => Diagnostic::error(
            DiagnosticKind::Eval,
            span,
            "unbound variable at runtime",
        )
        .with_code(ErrorCode::EvalUnboundVariable)
        .with_label(Label::new(span, "this variable is not defined"))
        .with_help("define the variable before using it, or check the spelling"),

        EvalError::TypeError(msg) => {
            let message = if msg.len() > 80 {
                format!("runtime type error: {:.77}...", msg)
            } else {
                format!("runtime type error: {}", msg)
            };
            Diagnostic::error(DiagnosticKind::Eval, span, message)
                .with_code(ErrorCode::EvalTypeError)
                .with_label(Label::new(span, "error occurred here"))
                .with_help("add a type annotation or runtime guard to handle this case")
        }

        EvalError::DivisionByZero => Diagnostic::error(
            DiagnosticKind::Eval,
            span,
            "division by zero",
        )
        .with_code(ErrorCode::DivisionByZero)
        .with_label(Label::new(span, "attempted to divide by zero"))
        .with_note("the right-hand operand of `/` or `%` evaluated to zero")
        .with_help("ensure the divisor is non-zero before dividing; consider using an `if` guard: `if divisor != 0 -> x / divisor else 0`"),

        EvalError::AssertionFailed(msg) => Diagnostic::error(
            DiagnosticKind::Eval,
            span,
            "assertion failed",
        )
        .with_code(ErrorCode::AssertionFailed)
        .with_label(Label::new(
            span,
            if msg.is_empty() {
                "assertion condition evaluated to false".to_string()
            } else {
                format!("assertion failed: {}", msg)
            },
        ))
        .with_note("assertions check that a condition is true at runtime")
        .with_help("check the assertion condition or add a descriptive message to clarify the expected state"),

        EvalError::PatternMatchFailed => Diagnostic::error(
            DiagnosticKind::Eval,
            span,
            "pattern match failed at runtime",
        )
        .with_code(ErrorCode::PatternMatchFailed)
        .with_label(Label::new(span, "no pattern matched the value"))
        .with_note("the value being matched did not fit any of the patterns")
        .with_help("add a wildcard `_` pattern as a catch-all to handle unexpected values"),

        EvalError::NotAFunction => Diagnostic::error(
            DiagnosticKind::Eval,
            span,
            "expected a function, found a non-callable value",
        )
        .with_code(ErrorCode::EvalNotAFunction)
        .with_label(Label::new(span, "this value cannot be called"))
        .with_help("check that the call target is actually a function"),

        EvalError::WrongArity => Diagnostic::error(
            DiagnosticKind::Eval,
            span,
            "wrong number of arguments at runtime",
        )
        .with_code(ErrorCode::EvalWrongArity)
        .with_label(Label::new(
            span,
            "the number of arguments does not match the function parameters",
        ))
        .with_help("check the function signature for the expected number of arguments"),

        EvalError::ParseDiagnostics {
            module,
            path: _,
            source_text,
            diagnostics,
        } => {
            // For parse errors in imported modules, return the first diagnostic
            // or a summary diagnostic
            if let Some(first_diag) = diagnostics.first() {
                let mut diag = first_diag.clone();
                diag = diag.with_note(format!("in imported module `{}`", module));
                diag
            } else {
                Diagnostic::error(
                    DiagnosticKind::Eval,
                    Span::DUMMY,
                    format!("parse error in module `{}`", module),
                )
                .with_code(ErrorCode::ModuleParseError)
                .with_note(format!("source: {}", source_text.chars().take(200).collect::<String>()))
            }
        }
    }
}

/// Try to extract a span from an error created during expression evaluation.
/// 尝试从表达式求值期间创建的错误中提取 span。
///
/// Many EvalError sites don't carry span info directly. This function provides
/// a fallback Span (from the expression that triggered the error).
/// 许多 EvalError 站点不直接携带 span 信息。此函数提供回退 Span
/// （来自触发错误的表达式）。
#[allow(dead_code)]
pub fn eval_error_with_fallback_span(error: &EvalError, current_expr_span: Span) -> Diagnostic {
    eval_error_to_diagnostic(error, current_expr_span)
}
