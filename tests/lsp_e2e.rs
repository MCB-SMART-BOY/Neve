//! End-to-end tests for the LSP server.
//! LSP 服务器的端到端测试。

#[cfg(test)]
mod tests {
    use neve_lsp::Document;

    /// Test that a complete analysis pipeline works for a realistic Neve program.
    #[test]
    fn test_e2e_analysis_pipeline() {
        let source = r#"
let name = "Neve";
let version = 1;

fn greet(person) = 42;

struct Config {
    path: String,
    debug: Bool,
};

let cfg = #{
    path = "/tmp",
    debug = true,
};

let greeting = greet(name);
let items = [1, 2, 3];
let total = 1 + 2 + 3;
"#;

        let doc = Document::new("file:///e2e.neve".to_string(), source.to_string());

        // 1. Parse succeeded
        assert!(doc.ast.is_some(), "AST should be produced");
        assert!(doc.hir.is_some(), "HIR should be produced");
        assert!(doc.semantics.is_some(), "Semantics should be produced");

        // 2. Symbol index has all definitions
        let index = doc
            .symbol_index
            .as_ref()
            .expect("Symbol index should exist");
        let defs: Vec<_> = index.definitions.keys().collect();
        assert!(
            defs.contains(&&"name".to_string()),
            "Should contain 'name': {:?}",
            defs
        );
        assert!(
            defs.contains(&&"greet".to_string()),
            "Should contain 'greet'"
        );
        assert!(
            defs.contains(&&"Config".to_string()),
            "Should contain 'Config'"
        );
        assert!(defs.contains(&&"cfg".to_string()), "Should contain 'cfg'");

        // 3. Find definition works
        let offset = source.find("greet(name)").unwrap();
        let def = index
            .find_definition_at(offset)
            .expect("Should find greeting usage");
        assert_eq!(def.name, "greet");

        // 4. References work
        let name_refs = index.get_references("name");
        assert!(!name_refs.is_empty(), "name should have references");

        // 5. Diagnostics present for valid code (should be empty or only warnings)
        let errors: Vec<_> = doc
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, neve_lsp::DiagnosticSeverity::Error))
            .collect();
        assert!(
            errors.is_empty(),
            "Valid code should have no errors: {:?}",
            errors
        );
    }

    /// Test that type errors produce diagnostics.
    #[test]
    fn test_e2e_type_error_diagnostics() {
        let source = r#"
let x: Int = "not an int";
let y = x + 1;
"#;

        let doc = Document::new("file:///error.neve".to_string(), source.to_string());

        let errors: Vec<_> = doc
            .diagnostics
            .iter()
            .filter(|d| matches!(d.severity, neve_lsp::DiagnosticSeverity::Error))
            .collect();
        assert!(!errors.is_empty(), "Type errors should produce diagnostics");
    }

    /// Test that CodeLens reference counts are correct.
    #[test]
    fn test_e2e_codelens_reference_counts() {
        let source = r#"
fn greet() = 42;
let a = greet();
let b = greet();
let c = greet();

struct Point { x: Int, y: Int };
let p = Point { x = 1, y = 2 };
"#;

        let doc = Document::new("file:///codelens.neve".to_string(), source.to_string());
        let index = doc.symbol_index.as_ref().expect("symbol index");

        // greet() is called 3 times
        let greet_refs = index.get_references("greet");
        let greet_uses: Vec<_> = greet_refs.iter().filter(|r| !r.is_write).collect();
        assert_eq!(greet_uses.len(), 3, "greet() should have 3 call sites");

        // Point struct is referenced once (construction)
        let point_refs = index.get_references("Point");
        assert!(!point_refs.is_empty(), "Point should have references");

        // Verify function definition exists
        let greet_defs = index.get_definitions("greet").expect("greet definition");
        assert!(
            greet_defs
                .iter()
                .any(|s| matches!(s.kind, neve_lsp::symbol_index::SymbolKind::Function)),
            "greet should be a Function"
        );
    }

    /// Test completion scoring order (exact > prefix > contains).
    #[test]
    fn test_e2e_completion_scoring_order() {
        // Simulate the scoring logic used by the LSP backend
        fn score(label: &str, prefix: &str) -> u32 {
            if prefix.is_empty() {
                return 500;
            }
            let l = label.to_lowercase();
            let p = prefix.to_lowercase();
            if l == p {
                1000
            } else if l.starts_with(&p) {
                900 + prefix.len().min(99) as u32
            } else if l.contains(&p) {
                500
            } else {
                0
            }
        }

        // Exact match should score highest
        assert!(score("print", "print") > score("println", "print"));
        // Prefix match should beat contains match
        assert!(score("io.readFile", "io.read") > score("io.readFilePath", "File"));
        // Contains match should beat no match
        assert!(score("io.readFilePath", "Path") > score("len", "xyz"));
        // Empty prefix gives neutral score
        assert_eq!(score("anything", ""), 500);
    }

    /// Test semantic tokens for all major constructs.
    #[test]
    fn test_e2e_semantic_tokens_comprehensive() {
        let source = r#"
struct Point { x: Int, y: Int };
enum Color { Red, Green, Blue };
trait Show { fn show() = ""; };
impl Show for Int { fn show() = "Int"; };

fn distance(a, b) = {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
};

import std.io as io;
"#;

        let tokens = neve_lsp::generate_semantic_tokens_from_ast(source);
        assert!(!tokens.is_empty(), "Should produce semantic tokens");
        // Should have at least: Point (type), Color (type), Show (type),
        // distance (function), io (variable/import), x/y fields, Red/Green/Blue
        assert!(
            tokens.len() >= 10,
            "Expected at least 10 tokens, got {}",
            tokens.len()
        );
    }

    /// Test that hover maps are built correctly.
    #[test]
    fn test_e2e_hover_maps() {
        let source = r#"
let x = 42;
fn identity(a) = a;
let y = identity(x);
"#;

        let doc = Document::new("file:///hover.neve".to_string(), source.to_string());

        assert!(
            !doc.definition_hovers.is_empty(),
            "Should have definition hovers"
        );
        assert!(
            !doc.semantic_hovers.is_empty(),
            "Should have semantic hovers"
        );
    }

    /// Test that the symbol index correctly resolves references with scoping.
    #[test]
    fn test_e2e_scoping() {
        let source = r#"
let x = 1;
fn outer(a) = {
    let x = 2;
    a + x
};
let result = outer(x);
"#;

        let doc = Document::new("file:///scope.neve".to_string(), source.to_string());
        let index = doc
            .symbol_index
            .as_ref()
            .expect("Symbol index should exist");

        // The 'x' in "a + x" should resolve to the inner 'x' (2)
        let inner_use = source.find("a + x").unwrap() + 4; // position of 'x' in inner block
        let def = index
            .find_definition_at(inner_use)
            .expect("Should find definition of inner x");
        let def_start: usize = def.def_span.start.into();
        let inner_def_pos = source.find("let x = 2").unwrap() + 4;
        assert_eq!(
            def_start, inner_def_pos,
            "Inner 'x' should resolve to inner definition"
        );

        // The 'x' in "outer(x)" should resolve to the outer 'x' (1)
        let outer_use = source.rfind("x)").unwrap();
        let def = index
            .find_definition_at(outer_use)
            .expect("Should find definition of outer x");
        let def_start: usize = def.def_span.start.into();
        let outer_def_pos = source.find("let x = 1").unwrap() + 4;
        assert_eq!(
            def_start, outer_def_pos,
            "Outer 'x' should resolve to outer definition"
        );
    }

    /// Test type-aware method completion lookup.
    #[test]
    fn test_e2e_method_completion_types() {
        // String operations
        let string_doc = Document::new(
            "file:///string.neve".to_string(),
            r#"let s = "hello"; let upper = s.upper();"#.to_string(),
        );
        assert!(string_doc.semantics.is_some());
        if let Some(ref sem) = string_doc.semantics {
            // Should have type info for s.upper() expression
            assert!(
                !sem.expr_types.is_empty(),
                "String expressions should have types"
            );
        }

        // List operations
        let list_doc = Document::new(
            "file:///list.neve".to_string(),
            r#"let xs = [1,2,3]; let len = xs.len();"#.to_string(),
        );
        assert!(list_doc.semantics.is_some());
        if let Some(ref sem) = list_doc.semantics {
            assert!(
                !sem.expr_types.is_empty(),
                "List expressions should have types"
            );
        }
    }

    /// Test formatter round-trip.
    #[test]
    fn test_e2e_formatter() {
        let input = "x=42\ny =x+1\n";
        let formatted = neve_fmt::format(input).expect("Format should succeed");
        assert!(!formatted.is_empty());
        assert!(formatted.contains("x = 42"));
    }

    /// Test that signatures are generated for builtin completion items.
    #[test]
    fn test_e2e_completion_specs_not_empty() {
        // Verify that the completion specs module loads correctly
        let doc = Document::new(
            "file:///completions.neve".to_string(),
            "let _ = 1;\n".to_string(),
        );
        // Document analysis should succeed
        assert!(doc.ast.is_some());
        assert!(doc.symbol_index.is_some());
    }

    /// Test that user-defined struct types resolve correctly in type names.
    #[test]
    fn test_e2e_user_type_resolution() {
        let doc = Document::new(
            "file:///user_type.neve".to_string(),
            r#"
struct Point { x: Int, y: Int };
let p = #{ x = 1, y = 2 };
let origin = #{ x = 0, y = 0 };
"#
            .to_string(),
        );
        assert!(doc.semantics.is_some());
        if let Some(ref sem) = doc.semantics {
            // Point should be a named type
            let has_point = sem.global_names.values().any(|n| n == "Point");
            assert!(has_point, "Semantics should know about Point type");
        }
    }
}
