//! Type error construction with detailed messages.
//! 类型错误构造，提供详细的错误信息。
//!
//! This module provides builders for constructing informative type error
//! diagnostics with helpful context and suggestions.
//! 本模块提供构建器，用于构造带有上下文信息和建议的类型错误诊断。

use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};
use neve_hir::{BinOp, Ty, TyKind, UnaryOp};

/// Format a type for display in error messages.
/// 格式化类型以在错误信息中显示。
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
            let parts: Vec<_> = fields
                .iter()
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
/// 格式化二元运算符以供显示。
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
/// 格式化一元运算符以供显示。
fn format_unaryop(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
    }
}

/// Builder for type mismatch errors.
/// 类型不匹配错误的构建器。
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
            diag = diag.with_label(Label::new(
                exp_span,
                format!("expected due to this (`{}`)", expected_str),
            ));
        }

        if let Some(found_span) = self.found_span
            && found_span != self.span
        {
            diag = diag.with_label(Label::new(
                found_span,
                format!("this has type `{}`", found_str),
            ));
        }

        // Add helpful notes based on types
        diag = add_type_mismatch_help(diag, &self.expected, &self.found);

        diag
    }
}

/// Add helpful notes for common type mismatches.
/// 为常见的类型不匹配添加有用的提示信息。
fn add_type_mismatch_help(mut diag: Diagnostic, expected: &Ty, found: &Ty) -> Diagnostic {
    match (&expected.kind, &found.kind) {
        // Int vs Float
        (TyKind::Int, TyKind::Float) => {
            diag =
                diag.with_help("use `toInt` to convert Float to Int, or change the expected type");
        }
        (TyKind::Float, TyKind::Int) => {
            diag = diag
                .with_help("use `toFloat` to convert Int to Float, or change the expected type");
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
/// 创建 if/else 分支类型不匹配的错误。
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
    .with_label(Label::new(
        then_span,
        format!("this branch has type `{}`", then_str),
    ))
    .with_label(Label::new(
        else_span,
        format!("this branch has type `{}`", else_str),
    ))
    .with_note("the `if` and `else` branches must have the same type")
    .with_help(
        "consider converting one branch to match the other, or changing both to a common type"
            .to_string(),
    )
}

/// Create an error for match arm type mismatch.
/// 创建 match 分支类型不匹配的错误。
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
    .with_label(Label::new(
        first_arm_span,
        format!("first arm has type `{}`", first_str),
    ))
    .with_label(Label::new(
        arm_span,
        format!("arm {} has type `{}`", arm_index + 1, arm_str),
    ))
    .with_note("all match arms must have the same type")
}

/// Create an error for binary operator type mismatch.
/// 创建二元运算符类型不匹配的错误。
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
        format!(
            "cannot apply `{}` to `{}` and `{}`",
            op_str, left_str, right_str
        ),
    )
    .with_code(ErrorCode::BinaryOpTypeMismatch)
    .with_label(Label::new(
        left_span,
        format!("this has type `{}`", left_str),
    ))
    .with_label(Label::new(
        right_span,
        format!("this has type `{}`", right_str),
    ));

    // Add operator-specific help
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
            diag = diag.with_note(format!(
                "the `{}` operator requires both operands to be numeric (Int or Float)",
                op_str
            ));
        }
        BinOp::Eq | BinOp::Ne => {
            diag = diag
                .with_note("the equality operators require both operands to have the same type");
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            diag = diag.with_note(
                "comparison operators require both operands to be of the same orderable type",
            );
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
            diag = diag.with_note(
                "the right side of `|>` must be a function that accepts the left side's type",
            );
        }
    }

    diag
}

/// Create an error for unary operator type mismatch.
/// 创建一元运算符类型不匹配的错误。
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
    .with_label(Label::new(
        operand_span,
        format!("this has type `{}`", ty_str),
    ));

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
/// 创建参数数量错误的错误。
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
        format!(
            "expected {} argument{}",
            expected,
            if expected == 1 { "" } else { "s" }
        ),
    ));

    if let Some(fs) = fn_span {
        diag = diag.with_label(Label::new(fs, format!("{} defined here", name)));
    }

    diag
}

/// Create an error for calling a non-function.
/// 创建调用非函数类型的错误。
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
/// 创建未绑定变量的错误。
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
/// 创建记录缺失字段的错误。
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
/// 创建记录中未知字段的错误。
pub fn unknown_field(field: &str, record_ty: &Ty, span: Span, available: &[String]) -> Diagnostic {
    let ty_str = format_type(record_ty);

    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("unknown field `{}` in record", field),
    )
    .with_code(ErrorCode::UnknownField)
    .with_label(Label::new(
        span,
        format!("field `{}` does not exist", field),
    ))
    .with_note(format!("record type is `{}`", ty_str));

    if !available.is_empty() {
        let available_str = available.join(", ");
        diag = diag.with_help(format!("available fields: {}", available_str));
    }

    diag
}

/// Create an error for missing trait method.
/// 创建缺失特征方法的错误。
pub fn missing_method(method: &str, trait_name: &str, impl_ty: &Ty, span: Span) -> Diagnostic {
    let ty_str = format_type(impl_ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!(
            "missing method `{}` in impl of `{}` for `{}`",
            method, trait_name, ty_str
        ),
    )
    .with_code(ErrorCode::MissingMethod)
    .with_label(Label::new(span, format!("missing `{}`", method)))
    .with_help(format!("implement the `{}` method", method))
}

/// Create an error for infinite type.
/// 创建无限类型的错误（类型变量出现在自身类型中）。
pub fn infinite_type(var: u32, ty: &Ty, span: Span) -> Diagnostic {
    let ty_str = format_type(ty);

    Diagnostic::error(DiagnosticKind::Type, span, "cannot construct infinite type")
        .with_code(ErrorCode::InfiniteType)
        .with_label(Label::new(
            span,
            format!("type variable `?{}` occurs in `{}`", var, ty_str),
        ))
        .with_note("this would create a recursive type that is infinite in size")
        .with_help("consider using an explicit type annotation or restructuring your code")
}

/// Create an error for pattern type mismatch.
/// 创建模式类型不匹配的错误。
pub fn pattern_mismatch(expected: &Ty, pattern_ty: &Ty, span: Span) -> Diagnostic {
    let expected_str = format_type(expected);
    let pattern_str = format_type(pattern_ty);

    Diagnostic::error(DiagnosticKind::Type, span, "pattern type mismatch")
        .with_code(ErrorCode::TypeMismatch)
        .with_label(Label::new(
            span,
            format!(
                "expected `{}`, found pattern for `{}`",
                expected_str, pattern_str
            ),
        ))
        .with_note("the pattern must match the type of the value being matched")
}

/// Create an error for non-exhaustive pattern match.
/// 创建模式匹配不完整的错误。
pub fn non_exhaustive_match(missing_patterns: &[String], span: Span) -> Diagnostic {
    let patterns_str = missing_patterns.join(", ");

    Diagnostic::error(DiagnosticKind::Type, span, "non-exhaustive pattern match")
        .with_code(ErrorCode::NonExhaustiveMatch)
        .with_label(Label::new(span, "patterns not covered"))
        .with_note(format!("missing patterns: {}", patterns_str))
        .with_help("add a wildcard pattern `_` or handle all cases explicitly")
}

/// Create an error for unreachable pattern.
/// 创建不可达模式的警告。
pub fn unreachable_pattern(span: Span, previous_span: Span) -> Diagnostic {
    Diagnostic::warning(DiagnosticKind::Type, span, "unreachable pattern")
        .with_label(Label::new(span, "this pattern will never be matched"))
        .with_label(Label::new(
            previous_span,
            "previous pattern matches all values",
        ))
        .with_help("remove this pattern or reorder the match arms")
}

/// Create an error for ambiguous type.
/// 创建类型模糊（无法推断）的错误。
pub fn ambiguous_type(span: Span, context: &str) -> Diagnostic {
    Diagnostic::error(DiagnosticKind::Type, span, "type annotations needed")
        .with_code(ErrorCode::AmbiguousType)
        .with_label(Label::new(span, "cannot infer type"))
        .with_note(format!("type must be known in {}", context))
        .with_help("add a type annotation to clarify the intended type")
}

/// Create an error for private access.
/// 创建访问私有成员的错误。
pub fn private_access(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(DiagnosticKind::Type, span, format!("`{}` is private", name))
        .with_code(ErrorCode::PrivateAccess)
        .with_label(Label::new(span, "private item"))
        .with_help(format!("consider making `{}` public with `pub`", name))
}

/// Create an error for cyclic dependency.
/// 创建循环依赖的错误。
pub fn cyclic_dependency(items: &[String], span: Span) -> Diagnostic {
    let cycle_str = items.join(" -> ");

    Diagnostic::error(DiagnosticKind::Type, span, "cyclic dependency detected")
        .with_code(ErrorCode::CyclicDependency)
        .with_label(Label::new(span, "cycle starts here"))
        .with_note(format!("cycle: {}", cycle_str))
        .with_help("break the cycle by restructuring your code")
}

/// Create an error for unused variable.
/// 创建未使用变量的警告。
pub fn unused_variable(name: &str, span: Span) -> Diagnostic {
    Diagnostic::warning(
        DiagnosticKind::Type,
        span,
        format!("unused variable: `{}`", name),
    )
    .with_label(Label::new(span, "this variable is never used"))
    .with_help(format!(
        "if this is intentional, prefix the name with an underscore: `_{}`",
        name
    ))
}

/// Create an error for redundant type annotation.
/// 创建冗余类型标注的警告。
pub fn redundant_annotation(inferred: &Ty, span: Span) -> Diagnostic {
    let ty_str = format_type(inferred);

    Diagnostic::warning(DiagnosticKind::Type, span, "redundant type annotation")
        .with_label(Label::new(
            span,
            format!("type `{}` can be inferred", ty_str),
        ))
        .with_help("consider removing the type annotation")
}

/// Find the most similar name to a given name from a list of candidates.
/// 从候选列表中找到与给定名称最相似的名称。
///
/// Uses Levenshtein distance for fuzzy matching.
/// 使用 Levenshtein 距离进行模糊匹配。
pub fn find_similar_name<'a>(name: &str, candidates: &'a [String]) -> Option<&'a str> {
    let max_distance = match name.len() {
        0..=2 => 0, // Very short names: exact match only / 非常短的名称：仅精确匹配
        3..=5 => 1, // Short names: 1 edit distance / 短名称：1 编辑距离
        _ => 2,     // Longer names: 2 edit distance / 较长名称：2 编辑距离
    };

    candidates
        .iter()
        .filter_map(|candidate| {
            let distance = levenshtein_distance(name, candidate);
            if distance <= max_distance {
                Some((candidate.as_str(), distance))
            } else {
                None
            }
        })
        .min_by_key(|(_, d)| *d)
        .map(|(s, _)| s)
}

/// Calculate the Levenshtein distance between two strings.
/// 计算两个字符串之间的 Levenshtein 距离。
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
        row[0] = i;
    }
    for (j, val) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
        *val = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Suggest a type conversion between two types.
/// 建议两种类型之间的类型转换。
pub fn suggest_conversion(from: &Ty, to: &Ty) -> Option<String> {
    match (&from.kind, &to.kind) {
        // Numeric conversions / 数值转换
        (TyKind::Int, TyKind::Float) => Some("toFloat(value)".to_string()),
        (TyKind::Float, TyKind::Int) => Some("toInt(value)".to_string()),

        // String conversions / 字符串转换
        (TyKind::Int, TyKind::String) => Some("toString(value)".to_string()),
        (TyKind::Float, TyKind::String) => Some("toString(value)".to_string()),
        (TyKind::Bool, TyKind::String) => Some("toString(value)".to_string()),
        (TyKind::Char, TyKind::String) => Some("toString(value)".to_string()),

        // Char to String / 字符到字符串
        (TyKind::String, TyKind::Char) => Some("use string indexing: str[0]".to_string()),

        // List to String / 列表到字符串
        (TyKind::Named(_, _), TyKind::String) => Some("use `join` or `toString`".to_string()),

        // Record field access / 记录字段访问
        (TyKind::Record(_), _) => Some("access a specific field with `.field`".to_string()),

        // Tuple element access / 元组元素访问
        (TyKind::Tuple(_), _) => {
            Some("access a specific element with `.0`, `.1`, etc.".to_string())
        }

        // Option/Result unwrap / Option/Result 解包
        (TyKind::Named(_, args), _) if !args.is_empty() => {
            Some("use `?` operator or pattern matching to unwrap".to_string())
        }

        _ => None,
    }
}

/// Create an error for accessing field on wrong type.
/// 创建在错误类型上访问字段的错误。
pub fn field_access_on_non_record(ty: &Ty, field: &str, span: Span) -> Diagnostic {
    let ty_str = format_type(ty);

    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("cannot access field `{}` on type `{}`", field, ty_str),
    )
    .with_code(ErrorCode::UnknownField)
    .with_label(Label::new(
        span,
        format!("type `{}` is not a record", ty_str),
    ));

    // Add context-specific help
    match &ty.kind {
        TyKind::Tuple(elems) => {
            diag = diag.with_note(format!(
                "tuples have {} elements, accessed with .0, .1, etc.",
                elems.len()
            ));
        }
        TyKind::Named(_, _) => {
            diag = diag.with_note("this is a named type, not a record");
        }
        TyKind::Fn(_, _) => {
            diag = diag.with_note(
                "functions don't have fields - did you mean to call this function first?",
            );
        }
        _ => {}
    }

    diag
}

/// Create an error for tuple index on wrong type.
/// 创建在错误类型上进行元组索引的错误。
pub fn tuple_index_on_non_tuple(ty: &Ty, index: u32, span: Span) -> Diagnostic {
    let ty_str = format_type(ty);

    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("cannot index into type `{}` with `.{}`", ty_str, index),
    )
    .with_code(ErrorCode::TypeMismatch)
    .with_label(Label::new(
        span,
        format!("type `{}` is not a tuple", ty_str),
    ));

    // Add context-specific help
    match &ty.kind {
        TyKind::Record(fields) => {
            let field_names: Vec<_> = fields.iter().map(|(n, _)| n.as_str()).collect();
            diag = diag
                .with_note("records are accessed with field names, not numeric indices")
                .with_help(format!("available fields: {}", field_names.join(", ")));
        }
        TyKind::Named(_, _) => {
            diag = diag.with_note("use pattern matching to destructure this type");
        }
        _ => {}
    }

    diag
}

/// Create an error for list index on wrong type.
/// 创建在错误类型上进行列表索引的错误。
pub fn list_index_on_non_list(ty: &Ty, span: Span) -> Diagnostic {
    let ty_str = format_type(ty);

    let mut diag = Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("cannot index into type `{}`", ty_str),
    )
    .with_code(ErrorCode::TypeMismatch)
    .with_label(Label::new(
        span,
        format!("type `{}` is not indexable", ty_str),
    ));

    // Add context-specific help
    match &ty.kind {
        TyKind::Tuple(elems) => {
            diag = diag.with_note(format!(
                "tuples use compile-time indices (.0, .1, ..., .{})",
                elems.len().saturating_sub(1)
            ));
        }
        TyKind::Record(_) => {
            diag = diag.with_note("records use field names, not indices");
        }
        TyKind::String => {
            diag = diag.with_help("strings can be indexed with [n] to get individual characters");
        }
        _ => {}
    }

    diag
}

/// Create an error for condition not being a boolean.
/// 创建条件不是布尔值的错误。
pub fn non_bool_condition(ty: &Ty, context: &str, span: Span) -> Diagnostic {
    let ty_str = format_type(ty);

    Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("{} condition must be a Bool", context),
    )
    .with_code(ErrorCode::TypeMismatch)
    .with_label(Label::new(
        span,
        format!("expected Bool, found `{}`", ty_str),
    ))
    .with_note(format!(
        "the condition in {} must evaluate to true or false",
        context
    ))
}

/// Create an error for using break/continue outside of a loop.
/// 创建在循环外使用 break/continue 的错误。
pub fn break_outside_loop(keyword: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        DiagnosticKind::Type,
        span,
        format!("`{}` outside of loop", keyword),
    )
    .with_code(ErrorCode::TypeMismatch)
    .with_label(Label::new(
        span,
        format!("cannot `{}` outside of a loop", keyword),
    ))
    .with_note(format!(
        "`{}` can only be used inside a loop (for, while, loop)",
        keyword
    ))
}

/// Create an error for return outside of a function.
/// 创建在函数外使用 return 的错误。
pub fn return_outside_function(span: Span) -> Diagnostic {
    Diagnostic::error(DiagnosticKind::Type, span, "`return` outside of function")
        .with_code(ErrorCode::TypeMismatch)
        .with_label(Label::new(span, "cannot return here"))
        .with_note("`return` can only be used inside a function body")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("a", ""), 1);
        assert_eq!(levenshtein_distance("", "a"), 1);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("abc", "abcd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_find_similar_name() {
        let candidates = vec![
            "println".to_string(),
            "print".to_string(),
            "printf".to_string(),
            "display".to_string(),
        ];

        assert_eq!(find_similar_name("prin", &candidates), Some("print"));
        assert_eq!(find_similar_name("printt", &candidates), Some("print"));
        assert_eq!(find_similar_name("xyz", &candidates), None);
    }
}
