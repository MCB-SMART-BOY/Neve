//! High-level Intermediate Representation for Neve.
//!
//! HIR is a desugared representation of the AST after name resolution.
//! It is used as input to the type checker.

mod hir;
mod lower;
mod resolve;

pub use hir::*;
pub use lower::lower;
pub use resolve::Resolver;

#[cfg(test)]
mod tests {
    use super::*;
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
            ItemKind::Fn(fn_def) => {
                match &fn_def.body.kind {
                    ExprKind::Binary(BinOp::Add, _, _) => {}
                    other => panic!("expected Binary Add, got {:?}", other),
                }
            }
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
            ItemKind::Fn(fn_def) => {
                match &fn_def.body.kind {
                    ExprKind::If(_, _, _) => {}
                    other => panic!("expected If, got {:?}", other),
                }
            }
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
            ItemKind::Fn(fn_def) => {
                match &fn_def.body.kind {
                    ExprKind::Match(_, arms) => {
                        assert_eq!(arms.len(), 2);
                    }
                    other => panic!("expected Match, got {:?}", other),
                }
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_variable_resolution() {
        let source = "fn f(x) = x + 1;";
        let (ast, diagnostics) = parse(source);
        assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);
        
        let hir = lower(&ast);
        assert_eq!(hir.items.len(), 1);
        
        match &hir.items[0].kind {
            ItemKind::Fn(fn_def) => {
                // The body should be x + 1
                match &fn_def.body.kind {
                    ExprKind::Binary(BinOp::Add, left, _) => {
                        // left should be a local variable reference
                        match &left.kind {
                            ExprKind::Var(local_id) => {
                                // Should reference the parameter
                                assert_eq!(fn_def.params[0].id, *local_id);
                            }
                            other => panic!("expected Var, got {:?}", other),
                        }
                    }
                    other => panic!("expected Binary Add, got {:?}", other),
                }
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_module_structure() {
        let source = "let x = 1; fn double(y) = y * 2;";
        let (ast, diagnostics) = parse(source);
        assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);
        
        let hir = lower(&ast);
        
        // Module should have name and id
        assert_eq!(hir.name, "main");
        assert_eq!(hir.id.0, 0);
        assert_eq!(hir.items.len(), 2);
    }

    #[test]
    fn test_module_registry() {
        let mut registry = ModuleRegistry::new();
        
        // Register a module
        let id = registry.register("std.list".to_string(), vec!["std".to_string(), "list".to_string()]);
        
        // Look it up
        let found = registry.lookup(&["std".to_string(), "list".to_string()]);
        assert_eq!(found, Some(id));
        
        // Not found
        let not_found = registry.lookup(&["std".to_string(), "map".to_string()]);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_import_parsing() {
        let source = "import std.list (map, filter);";
        let (ast, diagnostics) = parse(source);
        assert!(diagnostics.is_empty(), "parse errors: {:?}", diagnostics);
        
        let hir = lower(&ast);
        
        // Should have one import
        assert_eq!(hir.imports.len(), 1);
        assert_eq!(hir.imports[0].path, vec!["std", "list"]);
        
        match &hir.imports[0].kind {
            ImportKind::Items(items) => {
                assert_eq!(items, &vec!["map".to_string(), "filter".to_string()]);
            }
            _ => panic!("expected Items import"),
        }
    }
}
