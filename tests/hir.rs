//! Integration tests for neve-hir crate.

use neve_hir::{
    BUILTIN_OPTION_NONE_CTOR_ID, BUILTIN_OPTION_SOME_CTOR_ID, BinOp, ExprKind, ItemKind,
    PatternKind, StmtKind, TyKind, lower,
};
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
fn test_lower_function_preserves_explicit_generic_param_types() {
    let source = "fn id<T>(x: T) -> T = x;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => {
            assert_eq!(fn_def.generics.len(), 1);
            assert_eq!(fn_def.generics[0].name, "T");
            assert!(matches!(
                fn_def.params[0].ty.kind,
                TyKind::Param(0, ref name) if name == "T"
            ));
            assert!(matches!(
                fn_def.return_ty.kind,
                TyKind::Param(0, ref name) if name == "T"
            ));
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
fn test_lower_lambda_preserves_explicit_param_types() {
    let source = "let f = fn(x: Int) x;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Lambda(params, _) => {
                assert_eq!(params.len(), 1);
                assert!(matches!(params[0].ty.kind, TyKind::Int));
            }
            other => panic!("expected Lambda, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_block_let_preserves_tuple_pattern() {
    let source = "fn sum_pair() = { let (x, y) = (1, 2); x + y };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);
    assert_eq!(hir.items.len(), 1);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Block(stmts, _) => match &stmts[0].kind {
                StmtKind::Let { pattern, .. } => match &pattern.kind {
                    PatternKind::Tuple(items) => assert_eq!(items.len(), 2),
                    other => panic!("expected tuple pattern, got {:?}", other),
                },
                other => panic!("expected let stmt, got {:?}", other),
            },
            other => panic!("expected block, got {:?}", other),
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
fn test_lower_builtin_option_constructor_patterns_use_reserved_ids() {
    let source = "let x = match y { Some(v) -> v, None -> 0 };";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::Match(_, arms) => {
                assert_eq!(arms.len(), 2);
                match &arms[0].pattern.kind {
                    PatternKind::Constructor(def_id, args) => {
                        assert_eq!(*def_id, BUILTIN_OPTION_SOME_CTOR_ID);
                        assert_eq!(args.len(), 1);
                    }
                    other => panic!("expected constructor pattern, got {:?}", other),
                }
                match &arms[1].pattern.kind {
                    PatternKind::Constructor(def_id, args) => {
                        assert_eq!(*def_id, BUILTIN_OPTION_NONE_CTOR_ID);
                        assert!(args.is_empty());
                    }
                    other => panic!("expected constructor pattern, got {:?}", other),
                }
            }
            other => panic!("expected Match, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_self_and_assoc_type_use_sites() {
    let source = r#"
        trait Iterator {
            type Item;
            fn first(self) -> Self.Item;
        };
    "#;
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Trait(trait_def) => {
            assert_eq!(trait_def.items.len(), 1);
            assert!(matches!(trait_def.items[0].params[0].kind, TyKind::Unknown));
            assert!(matches!(
                trait_def.items[0].return_ty.kind,
                TyKind::SelfAssoc(ref name) if name == "Item"
            ));
        }
        other => panic!("expected trait item, got {:?}", other),
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

#[test]
fn test_lower_coalesce_expr_preserves_coalesce_node() {
    let source = "let x = value ?? fallback;";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => {
            assert!(matches!(fn_def.body.kind, ExprKind::Coalesce { .. }));
        }
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_method_call_preserves_method_node() {
    let source = "let x = 1.show();";
    let (ast, diagnostics) = parse(source);
    assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);

    let hir = lower(&ast);

    match &hir.items[0].kind {
        ItemKind::Fn(fn_def) => match &fn_def.body.kind {
            ExprKind::MethodCall { method, args, .. } => {
                assert_eq!(method, "show");
                assert!(args.is_empty());
            }
            other => panic!("expected MethodCall, got {:?}", other),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn test_lower_keeps_impl_method_ids_stable_when_appending_items() {
    let base = r#"
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
    "#;
    let appended = r#"
        trait Twice { fn twice(self) -> Int; };
        impl Twice for Int {
            fn twice(self) -> Int = self + self;
        };
        let x = 21.twice();
    "#;

    let (base_ast, base_diags) = parse(base);
    assert!(base_diags.is_empty(), "parse errors: {:?}", base_diags);
    let (appended_ast, appended_diags) = parse(appended);
    assert!(
        appended_diags.is_empty(),
        "parse errors: {:?}",
        appended_diags
    );

    let base_hir = lower(&base_ast);
    let appended_hir = lower(&appended_ast);

    let base_impl_id = match &base_hir.items[1].kind {
        ItemKind::Impl(def) => def.items[0].id,
        other => panic!("expected impl item, got {:?}", other),
    };
    let appended_impl_id = match &appended_hir.items[1].kind {
        ItemKind::Impl(def) => def.items[0].id,
        other => panic!("expected impl item, got {:?}", other),
    };

    assert_eq!(base_impl_id, appended_impl_id);
}
