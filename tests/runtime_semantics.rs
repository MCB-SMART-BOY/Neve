//! Regression coverage for lazy parameters and evaluator scope recovery.

use n3v3_eval::{EvalError, EvaluableModuleRef, Evaluator, Value};
use n3v3_frontend::analyze_source;
use n3v3_std::stdlib;

fn evaluate(source: &str) -> Result<Value, EvalError> {
    let analysis = analyze_source(source);
    let errors: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == n3v3_diagnostic::Severity::Error)
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

const SCOPED_LOCAL: n3v3_hir::LocalId = n3v3_hir::LocalId(0);

fn make_expr(kind: n3v3_hir::ExprKind) -> n3v3_hir::Expr {
    let span = n3v3_common::Span::default();
    n3v3_hir::Expr {
        kind,
        ty: n3v3_hir::Ty {
            kind: n3v3_hir::TyKind::Unknown,
            span,
        },
        span,
    }
}

fn make_int(value: i64) -> n3v3_hir::Expr {
    make_expr(n3v3_hir::ExprKind::Literal(n3v3_hir::Literal::Int(
        value.into(),
    )))
}

fn make_scoped_pattern() -> n3v3_hir::Pattern {
    n3v3_hir::Pattern {
        kind: n3v3_hir::PatternKind::Var(SCOPED_LOCAL, "scoped".to_string()),
        span: n3v3_common::Span::default(),
    }
}

fn make_failure() -> n3v3_hir::Expr {
    make_expr(n3v3_hir::ExprKind::Binary(
        n3v3_hir::BinOp::Div,
        Box::new(make_int(1)),
        Box::new(make_int(0)),
    ))
}

fn make_call(body: n3v3_hir::Expr) -> n3v3_hir::Expr {
    make_expr(n3v3_hir::ExprKind::Call(
        Box::new(make_expr(n3v3_hir::ExprKind::Lambda {
            params: Vec::new(),
            body: Box::new(body),
            return_ty: None,
        })),
        Vec::new(),
    ))
}

fn make_scoped_match(guard: Option<n3v3_hir::Expr>, body: n3v3_hir::Expr) -> n3v3_hir::Expr {
    make_expr(n3v3_hir::ExprKind::Match(
        Box::new(make_int(42)),
        vec![n3v3_hir::MatchArm {
            pattern: make_scoped_pattern(),
            guard,
            body,
            span: n3v3_common::Span::default(),
        }],
    ))
}

fn assert_scope_restored(expression: n3v3_hir::Expr, should_fail: bool) {
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
    let lookup = make_expr(n3v3_hir::ExprKind::Var(SCOPED_LOCAL));
    assert!(matches!(
        evaluator.eval(&lookup),
        Err(EvalError::UnboundVariable)
    ));
    assert_eq!(evaluator.eval(&make_int(7)).unwrap(), Value::Int(7.into()));
}

#[test]
fn block_error_does_not_leak_local_binding() {
    let expression = make_expr(n3v3_hir::ExprKind::Block(
        vec![n3v3_hir::Stmt {
            kind: n3v3_hir::StmtKind::Let {
                pattern: make_scoped_pattern(),
                ty: None,
                value: make_int(42),
            },
            span: n3v3_common::Span::default(),
        }],
        Some(Box::new(make_failure())),
    ));
    assert_scope_restored(expression.clone(), true);
    assert_scope_restored(make_call(expression), true);
}

#[test]
fn let_error_does_not_leak_local_binding() {
    assert_scope_restored(
        make_expr(n3v3_hir::ExprKind::Let {
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
    let body = make_call(make_expr(n3v3_hir::ExprKind::Var(SCOPED_LOCAL)));
    assert_scope_restored(make_call(make_scoped_match(None, body)), false);
}

#[test]
fn comprehension_condition_error_does_not_leak_local_binding() {
    assert_scope_restored(
        make_expr(n3v3_hir::ExprKind::ListComp {
            body: Box::new(make_int(42)),
            generators: vec![n3v3_hir::Generator {
                pattern: make_scoped_pattern(),
                iter: make_expr(n3v3_hir::ExprKind::List(vec![make_int(42)])),
                condition: Some(make_failure()),
                span: n3v3_common::Span::default(),
            }],
        }),
        true,
    );
}
