//! Integration tests for neve-hir crate.

use neve_hir::{BinOp, ExprKind, ItemKind, lower};
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
    let source = "let x = match 1 { 0 => false, _ => true };";
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
