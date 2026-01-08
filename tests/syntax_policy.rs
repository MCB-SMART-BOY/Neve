//! Syntax policy tests to protect "zero ambiguity" guarantees.

use neve_parser::parse;
use neve_syntax::{ExprKind, ItemKind};

fn parse_single_item(source: &str) -> neve_syntax::Item {
    let (file, diags) = parse(source);
    assert!(diags.is_empty(), "parse diagnostics: {diags:?}");
    assert_eq!(file.items.len(), 1, "expected a single item");
    file.items.into_iter().next().unwrap()
}

#[test]
fn record_literal_is_unambiguous() {
    let item = parse_single_item("let r = #{ x = 1 };");
    match item.kind {
        ItemKind::Let(def) => match def.value.kind {
            ExprKind::Record(_) => {}
            other => panic!("expected record literal, got {other:?}"),
        },
        other => panic!("expected let item, got {other:?}"),
    }
}

#[test]
fn block_expression_is_unambiguous() {
    let item = parse_single_item("let r = { let x = 1; x };");
    match item.kind {
        ItemKind::Let(def) => match def.value.kind {
            ExprKind::Block { .. } => {}
            other => panic!("expected block expression, got {other:?}"),
        },
        other => panic!("expected let item, got {other:?}"),
    }
}

#[test]
fn lambda_expression_is_unambiguous() {
    let item = parse_single_item("let f = fn(x) x + 1;");
    match item.kind {
        ItemKind::Let(def) => match def.value.kind {
            ExprKind::Lambda { .. } => {}
            other => panic!("expected lambda expression, got {other:?}"),
        },
        other => panic!("expected let item, got {other:?}"),
    }
}

#[test]
fn function_definition_is_unambiguous() {
    let item = parse_single_item("fn add(x) = x + 1;");
    match item.kind {
        ItemKind::Fn(_) => {}
        other => panic!("expected function item, got {other:?}"),
    }
}
