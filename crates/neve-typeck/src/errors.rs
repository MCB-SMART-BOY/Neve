//! Type error construction with detailed messages.
//!
//! This module provides builders for constructing informative type error
//! diagnostics with helpful context and suggestions.

use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};
use neve_hir::{Ty, TyKind, BinOp, UnaryOp};

/// Format a type for display in error messages.
pub fn format_type(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Int => "Int".to_string(),
        TyKind::Float => "Float".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::Char => "Char".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Unit => "()".to_string(),
        TyKind::Var(id) => format!("?{}", id),
        TyKind::Param(_, name) => name.clone(),
        TyKind::Named(def_id, args) => {
            if args.is_empty() {
                format!("Type#{}", def_id.0)
            } else {
                let args_str: Vec<_> = args.iter().map(format_type).collect();
                format!("Type#{}[{}]", def_id.0, args_str.join(", "))
            }
        }
        TyKind::Tuple(elems) => {
            let parts: Vec<_> = elems.iter().map(format_type).collect();
            format!("({})", parts.join(", "))
        }
        TyKind::Record(fields) => {
            let parts: Vec<_> = fields.iter()
                .map(|(name, ty)| format!("{}: {}", name, format_type(ty)))
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        TyKind::Fn(params, ret) => {
            let params_str: Vec<_> = params.iter().map(format_type).collect();
            format!("({}) -> {}", params_str.join(", "), format_type(ret))
        }
        TyKind::Forall(params, inner) => {
            format!("forall {}. {}", params.join(", "), format_type(inner))
        }
        TyKind::Unknown => "_".to_string(),
    }
}

/// Format a binary operator for display.
fn format_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::Pipe => "|>",
        BinOp::Concat => "++",
        BinOp::Merge => "//",
    }
}

/// Format a unary operator for display.
fn format_unaryop(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}

/// Builder for type mismatch errors.
pub struct TypeMismatchError {
    expected: Ty,
    found: Ty,
    span: Span,
    context: Option<String>,
    expected_span: Option<Span>,
    found_span: Option<Span>,
}

impl TypeMismatchError {
    pub fn new(expected: Ty, found: Ty, span: Span) -> Self {
        Self {
            expected,
            found,
            span,
            context: None,
            expected_span: None,
            found_span: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn with_expected_span(mut self, span: Span) -> Self {
        self.expected_span = Some(span);
        self
    }

    pub fn with_found_span(mut self, span: Span) -> Self {
        self.found_span = Some(span);
        self
    }

    pub fn build(self) -> Diagnostic {
        let expected_str = format_type(&self.expected);
        let found_str = format_type(&self.found);

        let message = if let Some(ctx) = &self.context {
            format!("type mismatch in {}", ctx)
        } else {
            "mismatched types".to_string()
        };

        let mut diag = Diagnostic::error(DiagnosticKind::Type, self.span, message)
            .with_code(ErrorCode::TypeMismatch)
            .with_label(Label::new(
                self.span,
                format!("expected `{}`, found `{}`", expected_str, found_str),
            ));

        if let Some(exp_span) = self.expected_span {
            diag = diag.with_label(Label::new(exp_span, format!("expected due to this (`{}`)", expected_str)));
        }

        if let Some(found_span) = self.found_span
            && found_span != self.span {
                diag = diag.with_label(Label::new(found_span, format!("this has type `{}`", found_str)));
            }

        // Add helpful notes based on types
        diag = add_type_mismatch_help(diag, &self.expected, &self.found);

        diag
    }
}

/// Add helpful notes for common type mismatches.
fn add_type_mismatch_help(mut diag: Diagnostic, expected: &Ty, found: &Ty) -> Diagnostic {
    match (&expected.kind, &found.kind) {
        // Int vs Float
        (TyKind::Int, TyKind::Float) => {
            diag = diag.with_help("use `toInt` to convert Float to Int, or change the expected type");
        }
        (TyKind::Float, TyKind::Int) => {
            diag = diag.with_help("use `toFloat` to convert Int to Float, or change the expected type");
        }
        // String vs Char
        (TyKind::String, TyKind::Char) => {
            diag = diag.with_help("use `toString` to convert Char to String");
        }
        (TyKind::Char, TyKind::String) => {
            diag = diag.with_note("a String is a sequence of Chars, not a single Char");
        }
        // Unit vs non-Unit
        (TyKind::Unit, _) => {
            diag = diag.with_note("this expression returns a value but one was not expected");
        }
        (_, TyKind::Unit) => {
            diag = diag.with_note("this expression does not return a value");
        }
        // Function mismatch
        (TyKind::Fn(exp_params, _), TyKind::Fn(found_params, _)) => {
            if exp_params.len() != found_params.len() {
                diag = diag.with_note(format!(
                    "expected function with {} parameter(s), found function with {} parameter(s)",
                    exp_params.len(),
                    found_params.len()
                ));
            }
        }
        // Tuple size mismatch
        (TyKind::Tuple(exp_elems), TyKind::Tuple(found_elems)) => {
            if exp_elems.len() != found_elems.len() {
                diag = diag.with_note(format!(
                    "expected tuple with {} element(s), found tuple with {} element(s)",
                    exp_elems.len(),
                    found_elems.len()
                ));
            }
        }
        _ => {}
    }

    diag
}

/// Create an error for if/else branch type mismatch.
pub fn if_branch_mismatch(
    then_ty: &Ty,
    else_ty: &Ty,
    if_span: Span,
    then_span: Span,
    else_span: Span,
) -> Diagnostic {
    let then_str = format_type(then_ty);
    let else_str = format_type(else_ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        if_span,
        "`if` and `else` have incompatible types",
    )
    .with_code(ErrorCode::IfBranchMismatch)
    .with_label(Label::new(then_span, format!("this branch has type `{}`", then_str)))
    .with_label(Label::new(else_span, format!("this branch has type `{}`", else_str)))
    .with_note("the `if` and `else` branches must have the same type")
    .with_help("consider converting one branch to match the other, or changing both to a common type".to_string())
}

/// Create an error for match arm type mismatch.
pub fn match_arm_mismatch(
    first_ty: &Ty,
    arm_ty: &Ty,
    match_span: Span,
    first_arm_span: Span,
    arm_span: Span,
    arm_index: usize,
) -> Diagnostic {
    let first_str = format_type(first_ty);
    let arm_str = format_type(arm_ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        match_span,
        "match arms have incompatible types",
    )
    .with_code(ErrorCode::MatchArmMismatch)
    .with_label(Label::new(first_arm_span, format!("first arm has type `{}`", first_str)))
    .with_label(Label::new(arm_span, format!("arm {} has type `{}`", arm_index + 1, arm_str)))
    .with_note("all match arms must have the same type")
}

/// Create an error for binary operator type mismatch.
pub fn binary_op_mismatch(
    op: &BinOp,
    left_ty: &Ty,
    right_ty: &Ty,
    op_span: Span,
    left_span: Span,
    right_span: Span,
) -> Diagnostic {
    let op_str = format_binop(op);
    let left_str = format_type(left_ty);
    let right_str = format_type(right_ty);

    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        op_span,
        format!("cannot apply `{}` to `{}` and `{}`", op_str, left_str, right_str),
    )
    .with_code(ErrorCode::BinaryOpTypeMismatch)
    .with_label(Label::new(left_span, format!("this has type `{}`", left_str)))
    .with_label(Label::new(right_span, format!("this has type `{}`", right_str)));

    // Add operator-specific help
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
            diag = diag.with_note(format!(
                "the `{}` operator requires both operands to be numeric (Int or Float)",
                op_str
            ));
        }
        BinOp::Eq | BinOp::Ne => {
            diag = diag.with_note("the equality operators require both operands to have the same type");
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            diag = diag.with_note("comparison operators require both operands to be of the same orderable type");
        }
        BinOp::And | BinOp::Or => {
            diag = diag.with_note("logical operators require both operands to be Bool");
        }
        BinOp::Concat => {
            diag = diag.with_note("the `++` operator works on lists and strings");
        }
        BinOp::Merge => {
            diag = diag.with_note("the `//` merge operator works on records");
        }
        BinOp::Pipe => {
            diag = diag.with_note("the right side of `|>` must be a function that accepts the left side's type");
        }
    }

    diag
}

/// Create an error for unary operator type mismatch.
pub fn unary_op_mismatch(
    op: &UnaryOp,
    operand_ty: &Ty,
    op_span: Span,
    operand_span: Span,
) -> Diagnostic {
    let op_str = format_unaryop(op);
    let ty_str = format_type(operand_ty);

    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        op_span,
        format!("cannot apply `{}` to `{}`", op_str, ty_str),
    )
    .with_code(ErrorCode::UnaryOpTypeMismatch)
    .with_label(Label::new(operand_span, format!("this has type `{}`", ty_str)));

    match op {
        UnaryOp::Neg => {
            diag = diag.with_note("the `-` operator requires a numeric type (Int or Float)");
        }
        UnaryOp::Not => {
            diag = diag.with_note("the `!` operator requires a Bool");
        }
    }

    diag
}

/// Create an error for wrong number of arguments.
pub fn wrong_arity(
    fn_name: Option<&str>,
    expected: usize,
    found: usize,
    call_span: Span,
    fn_span: Option<Span>,
) -> Diagnostic {
    let name = fn_name.unwrap_or("function");
    
    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        call_span,
        format!(
            "{} takes {} argument{} but {} {} supplied",
            name,
            expected,
            if expected == 1 { "" } else { "s" },
            found,
            if found == 1 { "was" } else { "were" }
        ),
    )
    .with_code(ErrorCode::WrongArity)
    .with_label(Label::new(
        call_span,
        format!("expected {} argument{}", expected, if expected == 1 { "" } else { "s" }),
    ));

    if let Some(fs) = fn_span {
        diag = diag.with_label(Label::new(fs, format!("{} defined here", name)));
    }

    diag
}

/// Create an error for calling a non-function.
pub fn not_a_function(ty: &Ty, span: Span) -> Diagnostic {
    let ty_str = format_type(ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("expected function, found `{}`", ty_str),
    )
    .with_code(ErrorCode::NotAFunction)
    .with_label(Label::new(span, format!("this has type `{}`", ty_str)))
    .with_note("only functions can be called")
}

/// Create an error for unbound variable.
pub fn unbound_variable(name: &str, span: Span, similar: Option<&str>) -> Diagnostic {
    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("cannot find value `{}` in this scope", name),
    )
    .with_code(ErrorCode::UnboundVariable)
    .with_label(Label::new(span, "not found in this scope"));

    if let Some(similar_name) = similar {
        diag = diag.with_help(format!("did you mean `{}`?", similar_name));
    }

    diag
}

/// Create an error for missing record field.
pub fn missing_field(field: &str, record_ty: &Ty, span: Span) -> Diagnostic {
    let ty_str = format_type(record_ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("missing field `{}` in record", field),
    )
    .with_code(ErrorCode::MissingField)
    .with_label(Label::new(span, format!("expected field `{}`", field)))
    .with_note(format!("record type is `{}`", ty_str))
}

/// Create an error for unknown record field.
pub fn unknown_field(field: &str, record_ty: &Ty, span: Span, available: &[String]) -> Diagnostic {
    let ty_str = format_type(record_ty);

    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("unknown field `{}` in record", field),
    )
    .with_code(ErrorCode::UnknownField)
    .with_label(Label::new(span, format!("field `{}` does not exist", field)))
    .with_note(format!("record type is `{}`", ty_str));

    if !available.is_empty() {
        let available_str = available.join(", ");
        diag = diag.with_help(format!("available fields: {}", available_str));
    }

    diag
}

/// Create an error for missing trait method.
pub fn missing_method(
    method: &str,
    trait_name: &str,
    impl_ty: &Ty,
    span: Span,
) -> Diagnostic {
    let ty_str = format_type(impl_ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("missing method `{}` in impl of `{}` for `{}`", method, trait_name, ty_str),
    )
    .with_code(ErrorCode::MissingMethod)
    .with_label(Label::new(span, format!("missing `{}`", method)))
    .with_help(format!("implement the `{}` method", method))
}

/// Create an error for infinite type.
pub fn infinite_type(var: u32, ty: &Ty, span: Span) -> Diagnostic {
    let ty_str = format_type(ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        span,
        "cannot construct infinite type",
    )
    .with_code(ErrorCode::InfiniteType)
    .with_label(Label::new(span, format!("type variable `?{}` occurs in `{}`", var, ty_str)))
    .with_note("this would create a recursive type that is infinite in size")
    .with_help("consider using an explicit type annotation or restructuring your code")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_type() {
        assert_eq!(format_type(&Ty { kind: TyKind::Int, span: Span::DUMMY }), "Int");
        assert_eq!(format_type(&Ty { kind: TyKind::Bool, span: Span::DUMMY }), "Bool");
        assert_eq!(format_type(&Ty { kind: TyKind::Unit, span: Span::DUMMY }), "()");
        
        let tuple = Ty {
            kind: TyKind::Tuple(vec![
                Ty { kind: TyKind::Int, span: Span::DUMMY },
                Ty { kind: TyKind::String, span: Span::DUMMY },
            ]),
            span: Span::DUMMY,
        };
        assert_eq!(format_type(&tuple), "(Int, String)");
    }

    #[test]
    fn test_type_mismatch_error() {
        let expected = Ty { kind: TyKind::Int, span: Span::DUMMY };
        let found = Ty { kind: TyKind::String, span: Span::DUMMY };
        
        let diag = TypeMismatchError::new(expected, found, Span::DUMMY)
            .with_context("function argument")
            .build();
        
        assert!(diag.message.contains("function argument"));
    }
}
