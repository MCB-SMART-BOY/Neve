//! Integration tests for neve-hir crate.

use neve_hir::{BinOp, ExprKind, ItemKind, PatternKind, lower};
use neve_parser::parse;

#[test]
fn test_lower_simple_let() {
    let source = "let x = 42;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => {
            assert_eq!(fn_def.name, "x");
            assert!(fn_def.params.is_empty());
        }
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_function() {
    let source = "fn double(x) = x * 2;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => {
            assert_eq!(fn_def.name, "double");
            assert_eq!(fn_def.params.len(), 1);
            assert_eq!(fn_def.params[0].name, "x");
        }
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_binary_expr() {
    let source = "let result = 1 + 2 * 3;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Binary(BinOp::Add, _, _) => {}
            other => panic!("expected Binary Add, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_if_expr() {
    let source = "let x = if true then 1 else 0;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::If(_, _, _) => {}
            other => panic!("expected If, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_match_expr() {
    let source = "let x = match 1 { 0 -> false, _ -> true };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Match(_, arms) => {
                assert_eq!(arms.len(), 2);
            }
            other => panic!("expected Match, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_lambda() {
    let source = "let f = fn(x) x + 1;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Lambda(params, _) => {
                assert_eq!(params.len(), 1);
            }
            other => panic!("expected Lambda, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_list() {
    let source = "let xs = [1, 2, 3];";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::List(items) => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected List, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_record() {
    let source = "let r = #{ x = 1, y = 2 };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Record(fields) => {
                assert_eq!(fields.len(), 2);
            }
            other => panic!("expected Record, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_or_pattern_preserves_shared_binding_ids() {
    let source = "let x = match (1, 2) { (0, v) | (1, v) -> v, _ -> 0 };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Match(_, arms) => match &arms[0].pattern.kind {
                PatternKind::Or(patterns) => {
                    assert_eq!(patterns.len(), 2);
                    match (&patterns[0].kind, &patterns[1].kind) {
                        (PatternKind::Tuple(lhs), PatternKind::Tuple(rhs)) => {
                            match (&lhs[1].kind, &rhs[1].kind) {
                                (
                                    PatternKind::Var(lhs_id, lhs_name),
                                    PatternKind::Var(rhs_id, rhs_name),
                                ) => {
                                    assert_eq!(lhs_name, "v");
                                    assert_eq!(rhs_name, "v");
                                    assert_eq!(
                                        lhs_id, rhs_id,
                                        "or-pattern alternatives must reuse the same LocalId"
                                    );
                                }
                                other => {
                                    panic!("expected shared variable bindings, got {:?}", other)
                                }
                            }
                        }
                        other => panic!("expected tuple alternatives, got {:?}", other),
                    }
                }
                other => panic!("expected Or pattern, got {:?}", other),
            },
            other => panic!("expected Match, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_binding_pattern_preserves_inner_pattern() {
    let source = "let x = match 42 { n @ 42 -> n, _ -> 0 };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Match(_, arms) => match &arms[0].pattern.kind {
                PatternKind::Binding(_, name, inner) => {
                    assert_eq!(name, "n");
                    assert!(matches!(inner.kind, PatternKind::Literal(_)));
                }
                other => panic!("expected Binding pattern, got {:?}", other),
            },
            other => panic!("expected Match, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_list_rest_pattern_preserves_segments() {
    let source = "let x = match [1, 2, 3, 4] { [first, ..middle, last] -> first, _ -> 0 };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Match(_, arms) => match &arms[0].pattern.kind {
                PatternKind::ListRest { init, rest, tail } => {
                    assert_eq!(init.len(), 1);
                    assert_eq!(tail.len(), 1);
                    match rest.as_deref() {
                        Some(neve_hir::Pattern {
                            kind: PatternKind::Var(_, name),
                            ..
                        }) => assert_eq!(name, "middle"),
                        other => panic!("expected middle rest binding, got {:?}", other),
                    }
                }
                other => panic!("expected ListRest pattern, got {:?}", other),
            },
            other => panic!("expected Match, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_try_expr_preserves_try_node() {
    let source = "let x = value?;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => {
            assert!(matches!(fn_def.body.kind, ExprKind::Try(_)));
        }
        _ => panic!("expected function"),
    }
}
