//! Regression coverage for lazy parameters and evaluator scope recovery.

use neve_eval::{EvalError, EvaluableModuleRef, Evaluator, Value};
use neve_frontend::analyze_source;
use neve_std::stdlib;

fn evaluate(source: &str) -> Result<Value, EvalError> {
    let analysis = analyze_source(source);
    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == neve_diagnostic::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected diagnostics: {errors:?}");
    let mut evaluator = Evaluator::new().with_extra_builtins(
        stdlib()
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );
    evaluator.eval_evaluable_module(EvaluableModuleRef::new(
        &analysis.hir,
        &analysis.semantics.method_resolutions,
    ))
}

#[test]
fn lazy_parameter_unused_argument_does_not_evaluate() {
    let result = evaluate("fn discard(~value: Int) -> Int = 42; discard(1 / 0)");
    assert_eq!(result.unwrap(), Value::Int(42.into()));
}

#[test]
fn lazy_parameter_tail_call_does_not_evaluate_unused_argument() {
    let result = evaluate(
        "fn discard(~value: Int) -> Int = 42; fn relay(value: Int) -> Int = discard(value / 0); relay(1)",
    );
    assert_eq!(result.unwrap(), Value::Int(42.into()));
}

#[test]
fn lazy_parameter_required_argument_propagates_error() {
    let result = evaluate("fn demand(~value: Int) -> Int = value + 1; demand(1 / 0)");
    assert!(matches!(result, Err(EvalError::DivisionByZero)));
}

#[test]
fn lazy_parameter_tail_condition_selects_false_branch() {
    let result = evaluate("fn choose(~cond: Bool) -> Int = if cond -> 1 else 2; choose(false)");
    assert_eq!(result.unwrap(), Value::Int(2.into()));
}

#[test]
fn lazy_parameter_non_tail_condition_selects_false_branch() {
    let result =
        evaluate("fn choose(~cond: Bool) -> Int = (if cond -> 1 else 2) + 10; choose(false)");
    assert_eq!(result.unwrap(), Value::Int(12.into()));
}

#[test]
fn lazy_parameter_match_guard_selects_fallback() {
    let result = evaluate(
        "fn choose(~cond: Bool) -> Int = match true { true if cond -> 1, _ -> 2 }; choose(false)",
    );
    assert_eq!(result.unwrap(), Value::Int(2.into()));
}

#[test]
fn lazy_parameter_comprehension_condition_filters_items() {
    let result =
        evaluate("fn choose(~cond: Bool) -> List<Int> = [x | x <- [1, 2], cond]; choose(false)");
    assert_eq!(result.unwrap(), Value::List(std::rc::Rc::new(Vec::new())));
}

#[test]
fn lazy_parameter_method_argument_is_not_evaluated() {
    let result = evaluate(
        "trait Keep { fn keep(self, ~other: Int) -> Int; }; impl Keep for Int { fn keep(self, ~other: Int) -> Int = self; }; 42.keep(1 / 0)",
    );
    assert_eq!(result.unwrap(), Value::Int(42.into()));
}

#[test]
fn lazy_parameter_return_preserves_explicit_thunk() {
    let result = evaluate("fn suspend(~value: Int) = value; isLazy(suspend(1 / 0))");
    assert_eq!(result.unwrap(), Value::Bool(true));
}

#[test]
fn lazy_parameter_captures_callers_local_binding() {
    let result = evaluate(
        "fn demand(~value: Int) -> Int = value + 1; fn caller(value: Int) -> Int = demand(value); caller(41)",
    );
    assert_eq!(result.unwrap(), Value::Int(42.into()));
}

const SCOPED_LOCAL: neve_hir::LocalId = neve_hir::LocalId(0);

fn make_expr(kind: neve_hir::ExprKind) -> neve_hir::Expr {
    let span = neve_common::Span::default();
    neve_hir::Expr {
        kind,
        ty: neve_hir::Ty {
            kind: neve_hir::TyKind::Unknown,
            span,
        },
        span,
    }
}

fn make_int(value: i64) -> neve_hir::Expr {
    make_expr(neve_hir::ExprKind::Literal(neve_hir::Literal::Int(
        value.into(),
    )))
}

fn make_scoped_pattern() -> neve_hir::Pattern {
    neve_hir::Pattern {
        kind: neve_hir::PatternKind::Var(SCOPED_LOCAL, "scoped".to_string()),
        span: neve_common::Span::default(),
    }
}

fn make_failure() -> neve_hir::Expr {
    make_expr(neve_hir::ExprKind::Binary(
        neve_hir::BinOp::Div,
        Box::new(make_int(1)),
        Box::new(make_int(0)),
    ))
}

fn make_call(body: neve_hir::Expr) -> neve_hir::Expr {
    make_expr(neve_hir::ExprKind::Call(
        Box::new(make_expr(neve_hir::ExprKind::Lambda {
            params: Vec::new(),
            body: Box::new(body),
            return_ty: None,
        })),
        Vec::new(),
    ))
}

fn make_scoped_match(guard: Option<neve_hir::Expr>, body: neve_hir::Expr) -> neve_hir::Expr {
    make_expr(neve_hir::ExprKind::Match(
        Box::new(make_int(42)),
        vec![neve_hir::MatchArm {
            pattern: make_scoped_pattern(),
            guard,
            body,
            span: neve_common::Span::default(),
        }],
    ))
}

fn assert_scope_restored(expression: neve_hir::Expr, should_fail: bool) {
    let mut evaluator = Evaluator::new();
    let result = evaluator.eval(&expression);
    if should_fail {
        assert!(
            matches!(result, Err(EvalError::DivisionByZero)),
            "{result:?}"
        );
    } else {
        assert_eq!(result.unwrap(), Value::Int(42.into()));
    }
    let lookup = make_expr(neve_hir::ExprKind::Var(SCOPED_LOCAL));
    assert!(matches!(
        evaluator.eval(&lookup),
        Err(EvalError::UnboundVariable)
    ));
    assert_eq!(evaluator.eval(&make_int(7)).unwrap(), Value::Int(7.into()));
}

#[test]
fn block_error_does_not_leak_local_binding() {
    let expression = make_expr(neve_hir::ExprKind::Block(
        vec![neve_hir::Stmt {
            kind: neve_hir::StmtKind::Let {
                pattern: make_scoped_pattern(),
                ty: None,
                value: make_int(42),
            },
            span: neve_common::Span::default(),
        }],
        Some(Box::new(make_failure())),
    ));
    assert_scope_restored(expression.clone(), true);
    assert_scope_restored(make_call(expression), true);
}

#[test]
fn let_error_does_not_leak_local_binding() {
    assert_scope_restored(
        make_expr(neve_hir::ExprKind::Let {
            pattern: make_scoped_pattern(),
            ty: None,
            value: Box::new(make_int(42)),
            body: Box::new(make_failure()),
        }),
        true,
    );
}

#[test]
fn match_guard_error_does_not_leak_local_binding() {
    let expression = make_scoped_match(Some(make_failure()), make_int(42));
    assert_scope_restored(expression.clone(), true);
    assert_scope_restored(make_call(expression), true);
}

#[test]
fn match_body_error_does_not_leak_local_binding() {
    let expression = make_scoped_match(None, make_failure());
    assert_scope_restored(expression.clone(), true);
    assert_scope_restored(make_call(expression), true);
}

#[test]
fn match_tail_call_preserves_argument_and_restores_scope() {
    let body = make_call(make_expr(neve_hir::ExprKind::Var(SCOPED_LOCAL)));
    assert_scope_restored(make_call(make_scoped_match(None, body)), false);
}

#[test]
fn comprehension_condition_error_does_not_leak_local_binding() {
    assert_scope_restored(
        make_expr(neve_hir::ExprKind::ListComp {
            body: Box::new(make_int(42)),
            generators: vec![neve_hir::Generator {
                pattern: make_scoped_pattern(),
                iter: make_expr(neve_hir::ExprKind::List(vec![make_int(42)])),
                condition: Some(make_failure()),
                span: neve_common::Span::default(),
            }],
        }),
        true,
    );
}
